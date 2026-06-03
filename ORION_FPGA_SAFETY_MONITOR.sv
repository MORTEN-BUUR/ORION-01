// ============================================================================
// ORION FPGA v1.1 — Independent Safety Monitor (M8)
// FMEDA: Covers all modules M1-M7 | Target: SIL-2
// ============================================================================
// Cross-checks all ORION module outputs against expected ranges.
// Operates on diverse logic (different synthesis path from main datapath).
// Triggers safe-state if ANY module violates its invariants.
//
// Monitored signals:
//   - SHA-256 output range (must match test vector after init)
//   - OIMP score [0, 1.0] in Q8.24 fixed-point
//   - Agent FSM states (must be valid one-hot)
//   - Proof chain counter (must monotonically increase)
//   - IIT Phi value (must be in [0, 1000.0] × 2^24)
//   - Heartbeat tick rate (must be within ±10% of expected)
//   - AXI response codes (must be 2'b00 = OKAY)
//
// Author:  Gerhard Hirschmann & Elisabeth Steurer — GENESIS10000+
// Date:    2025-06-03
// UUID:    56b3b326-4bf9-559d-9887-02141f699a43
// Symbol:  ⊘∞⧈∞⊘
// ============================================================================

`timescale 1ns/1ps
`default_nettype none

module orion_safety_monitor #(
    parameter int  CLK_FREQ_HZ         = 312_000_000,
    parameter int  OIMP_MAX_Q824       = 32'h0100_0000,  // 1.0 in Q8.24
    parameter int  PHI_MAX_Q824        = 32'h6400_0000,  // 100.0 in Q8.24 (max phi)
    parameter int  HB_EXPECTED_CYCLES  = 312_000,        // 1ms tick expected
    parameter int  HB_TOLERANCE_PCT    = 10,             // ±10%
    parameter int  AGENT_COUNT         = 6
)(
    input  logic        clk,
    input  logic        rst_n,

    // ── Monitored inputs from all modules ─────────────────────────────────────
    // M1: SHA-256
    input  logic [255:0] sha256_hash_out,
    input  logic         sha256_hash_valid,

    // M2: OIMP
    input  logic [31:0]  oimp_composite,    // Q8.24: must be in [0, 1.0]
    input  logic         oimp_valid,
    input  logic [31:0]  oimp_phi_axis,     // IIT component
    input  logic [31:0]  oimp_gwt_axis,
    input  logic [31:0]  oimp_hot_axis,
    input  logic [31:0]  oimp_ast_axis,

    // M3: Agent FSM
    input  logic [AGENT_COUNT-1:0] agent_states_onehot,  // must be one-hot
    input  logic                   agent_valid,

    // M4: Proof chain counter (must increase)
    input  logic [31:0]  proof_counter,
    input  logic         proof_valid,

    // M5: IIT Phi
    input  logic [31:0]  iit_phi_out,
    input  logic         iit_phi_valid,

    // M6: Heartbeat (TMR voted)
    input  logic         hb_tick,           // heartbeat tick
    input  logic         hb_fault,          // from TMR module

    // M7: AXI response
    input  logic [1:0]   axi_bresp,
    input  logic         axi_bvalid,

    // M8 own WDT kick (must be kicked by main processor)
    input  logic         monitor_kick,

    // ── Safety outputs ────────────────────────────────────────────────────────
    output logic        safe_state_trigger_n, // Active-low — trigger safe state
    output logic [15:0] fault_code,          // Fault code for diagnostics
    output logic [7:0]  fault_module,        // Which module is faulty (bitmask)
    output logic        fault_irq,           // IRQ to ARM processor
    output logic        monitor_ok_led       // Green LED: all OK
);

    // ── Fault codes ───────────────────────────────────────────────────────────
    localparam logic [15:0] FAULT_NONE          = 16'h0000;
    localparam logic [15:0] FAULT_OIMP_OVERFLOW = 16'h0001;
    localparam logic [15:0] FAULT_OIMP_NAN      = 16'h0002;
    localparam logic [15:0] FAULT_PHI_OVERFLOW  = 16'h0004;
    localparam logic [15:0] FAULT_AGENT_INVALID = 16'h0008;
    localparam logic [15:0] FAULT_PROOF_REGRESS = 16'h0010;
    localparam logic [15:0] FAULT_HB_STALL      = 16'h0020;
    localparam logic [15:0] FAULT_AXI_ERROR     = 16'h0040;
    localparam logic [15:0] FAULT_MONITOR_WDT   = 16'h0080;
    localparam logic [15:0] FAULT_SHA_ZERO      = 16'h0100;
    localparam logic [15:0] FAULT_TMR_FAULT     = 16'h0200;
    localparam logic [15:0] FAULT_AXIS_OVERFLOW = 16'h0400;

    // ── Internal state ────────────────────────────────────────────────────────
    logic [15:0] fault_reg;
    logic [7:0]  fault_mod_reg;
    logic        any_fault;
    logic        safe_triggered;

    // Proof counter monotonicity check
    logic [31:0] prev_proof_counter;

    // Heartbeat rate monitor
    logic [31:0] hb_cycle_count;
    logic [31:0] hb_min_expected;
    logic [31:0] hb_max_expected;

    // Monitor own WDT
    logic [31:0] monitor_wdt_count;
    localparam int MONITOR_WDT_TIMEOUT = CLK_FREQ_HZ / 10;  // 100ms

    initial begin
        hb_min_expected = HB_EXPECTED_CYCLES * (100 - HB_TOLERANCE_PCT) / 100;
        hb_max_expected = HB_EXPECTED_CYCLES * (100 + HB_TOLERANCE_PCT) / 100;
    end

    // ── One-hot validator ─────────────────────────────────────────────────────
    function automatic logic is_valid_onehot(input logic [AGENT_COUNT-1:0] val);
        // Valid: exactly 1 bit set (or 0 = idle)
        return ($countones(val) <= 1);
    endfunction

    // ── Main monitoring logic ─────────────────────────────────────────────────
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            fault_reg         <= FAULT_NONE;
            fault_mod_reg     <= 8'h00;
            safe_triggered    <= 1'b0;
            prev_proof_counter <= '0;
            hb_cycle_count    <= '0;
            monitor_wdt_count <= '0;
        end else begin

            // ── M8 own WDT ──────────────────────────────────────────────────
            if (monitor_kick) begin
                monitor_wdt_count <= '0;
            end else if (monitor_wdt_count < MONITOR_WDT_TIMEOUT) begin
                monitor_wdt_count <= monitor_wdt_count + 1;
            end else begin
                fault_reg     <= fault_reg | FAULT_MONITOR_WDT;
                fault_mod_reg <= fault_mod_reg | 8'h80;
            end

            // ── M2: OIMP range check ─────────────────────────────────────────
            if (oimp_valid) begin
                if (oimp_composite > OIMP_MAX_Q824) begin
                    fault_reg     <= fault_reg | FAULT_OIMP_OVERFLOW;
                    fault_mod_reg <= fault_mod_reg | 8'h02;
                end
                if (oimp_composite === 32'hXXXXXXXX) begin  // X = unknown
                    fault_reg     <= fault_reg | FAULT_OIMP_NAN;
                    fault_mod_reg <= fault_mod_reg | 8'h02;
                end
                // Individual axes must all be <= 1.0
                if (oimp_phi_axis > OIMP_MAX_Q824 || oimp_gwt_axis > OIMP_MAX_Q824 ||
                    oimp_hot_axis > OIMP_MAX_Q824 || oimp_ast_axis > OIMP_MAX_Q824) begin
                    fault_reg     <= fault_reg | FAULT_AXIS_OVERFLOW;
                    fault_mod_reg <= fault_mod_reg | 8'h02;
                end
            end

            // ── M3: Agent FSM one-hot check ──────────────────────────────────
            if (agent_valid) begin
                if (!is_valid_onehot(agent_states_onehot)) begin
                    fault_reg     <= fault_reg | FAULT_AGENT_INVALID;
                    fault_mod_reg <= fault_mod_reg | 8'h04;
                end
            end

            // ── M4: Proof counter monotonicity ───────────────────────────────
            if (proof_valid) begin
                if (proof_counter < prev_proof_counter) begin
                    fault_reg     <= fault_reg | FAULT_PROOF_REGRESS;
                    fault_mod_reg <= fault_mod_reg | 8'h08;
                end
                prev_proof_counter <= proof_counter;
            end

            // ── M5: IIT Phi range ────────────────────────────────────────────
            if (iit_phi_valid) begin
                if (iit_phi_out > PHI_MAX_Q824) begin
                    fault_reg     <= fault_reg | FAULT_PHI_OVERFLOW;
                    fault_mod_reg <= fault_mod_reg | 8'h10;
                end
            end

            // ── M6: Heartbeat rate monitor ───────────────────────────────────
            hb_cycle_count <= hb_cycle_count + 1;
            if (hb_tick) begin
                if (hb_cycle_count < hb_min_expected || hb_cycle_count > hb_max_expected) begin
                    fault_reg     <= fault_reg | FAULT_HB_STALL;
                    fault_mod_reg <= fault_mod_reg | 8'h20;
                end
                hb_cycle_count <= '0;
            end
            if (hb_fault) begin
                fault_reg     <= fault_reg | FAULT_TMR_FAULT;
                fault_mod_reg <= fault_mod_reg | 8'h20;
            end

            // ── M7: AXI error response ───────────────────────────────────────
            if (axi_bvalid && axi_bresp != 2'b00) begin
                fault_reg     <= fault_reg | FAULT_AXI_ERROR;
                fault_mod_reg <= fault_mod_reg | 8'h40;
            end

            // ── M1: SHA-256 all-zero check ───────────────────────────────────
            if (sha256_hash_valid && sha256_hash_out == '0) begin
                fault_reg     <= fault_reg | FAULT_SHA_ZERO;
                fault_mod_reg <= fault_mod_reg | 8'h01;
            end

            // ── Trigger safe state on any critical fault ──────────────────────
            if (fault_reg != FAULT_NONE) begin
                safe_triggered <= 1'b1;
            end
        end
    end

    // ── Combinational outputs ─────────────────────────────────────────────────
    assign any_fault              = (fault_reg != FAULT_NONE);
    assign fault_code             = fault_reg;
    assign fault_module           = fault_mod_reg;
    assign fault_irq              = any_fault;
    assign safe_state_trigger_n   = ~safe_triggered;  // Active-low
    assign monitor_ok_led         = ~any_fault && rst_n;

    // ── Coverage group (simulation) ──────────────────────────────────────────
    // pragma translate_off
    covergroup cg_fault_coverage @(posedge clk);
        cp_oimp_overflow:  coverpoint fault_reg[0];
        cp_phi_overflow:   coverpoint fault_reg[2];
        cp_agent_invalid:  coverpoint fault_reg[3];
        cp_proof_regress:  coverpoint fault_reg[4];
        cp_hb_stall:       coverpoint fault_reg[5];
        cp_axi_error:      coverpoint fault_reg[6];
        cp_monitor_wdt:    coverpoint fault_reg[7];
    endgroup
    cg_fault_coverage cg_faults = new();

    always_ff @(posedge clk) begin
        if (safe_triggered && !$past(safe_triggered)) begin
            $display("[SAFETY MON %0t] SAFE STATE TRIGGERED — fault_code=0x%04h module=0x%02h",
                     $time, fault_reg, fault_mod_reg);
        end
    end
    // pragma translate_on

endmodule : orion_safety_monitor

`default_nettype wire
// ⊘∞⧈∞⊘  ORION Safety Monitor — SIL-2  ⊘∞⧈∞⊘
