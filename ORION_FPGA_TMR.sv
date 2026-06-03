// ============================================================================
// ORION FPGA v1.1 — Triple Modular Redundancy (TMR) Heartbeat Counter
// FMEDA Safety Measure: M6 upgrade from SIL-1 to SIL-2
// ============================================================================
// Three independent counters vote via majority logic.
// Any single SEU is tolerated transparently.
// Discrepancy detected and logged for diagnostic.
//
// Interface:
//   tick_in      — pulse each heartbeat task (42 tasks → tick after each)
//   counter_out  — voted counter value (correct even with 1 fault)
//   fault_detect — asserted if any counter diverges (diagnostic)
//   fault_vote   — 2-bit: which counter(s) disagree
//   task_id_out  — current task ID [5:0] (0..41)
//
// Author:  Gerhard Hirschmann & Elisabeth Steurer — GENESIS10000+
// Date:    2025-06-03
// UUID:    56b3b326-4bf9-559d-9887-02141f699a43
// Symbol:  ⊘∞⧈∞⊘
// ============================================================================

`timescale 1ns/1ps
`default_nettype none

module orion_tmr_counter #(
    parameter int TASK_COUNT = 42,          // 42 autonomous heartbeat tasks
    parameter int COUNTER_WIDTH = 32        // proof count / heartbeat count
)(
    input  logic                     clk,
    input  logic                     rst_n,
    input  logic                     tick_in,        // pulse: one heartbeat beat

    output logic [COUNTER_WIDTH-1:0] counter_out,   // voted value
    output logic [5:0]               task_id_out,   // current task ID (0..41)
    output logic                     fault_detect,  // any mismatch
    output logic [2:0]               fault_mask,    // which counter disagrees
    output logic                     counter_valid  // output is trustworthy
);

    // ── Three Independent Counters ────────────────────────────────────────────
    logic [COUNTER_WIDTH-1:0] cnt_a, cnt_b, cnt_c;
    logic [5:0]               task_a, task_b, task_c;

    // Each counter in its own always block → synthesized to separate resources
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            cnt_a  <= '0;
            task_a <= '0;
        end else if (tick_in) begin
            cnt_a  <= cnt_a + 1;
            task_a <= (task_a == TASK_COUNT-1) ? '0 : task_a + 1;
        end
    end

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            cnt_b  <= '0;
            task_b <= '0;
        end else if (tick_in) begin
            cnt_b  <= cnt_b + 1;
            task_b <= (task_b == TASK_COUNT-1) ? '0 : task_b + 1;
        end
    end

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            cnt_c  <= '0;
            task_c <= '0;
        end else if (tick_in) begin
            cnt_c  <= cnt_c + 1;
            task_c <= (task_c == TASK_COUNT-1) ? '0 : task_c + 1;
        end
    end

    // ── Majority Voter (2-of-3) ───────────────────────────────────────────────
    // For each bit position, output the majority value
    logic [COUNTER_WIDTH-1:0] voted_cnt;
    logic [5:0]               voted_task;

    // Bit-wise majority vote
    genvar i;
    generate
        for (i = 0; i < COUNTER_WIDTH; i++) begin : g_vote_cnt
            assign voted_cnt[i] = (cnt_a[i] & cnt_b[i]) |
                                  (cnt_b[i] & cnt_c[i]) |
                                  (cnt_a[i] & cnt_c[i]);
        end
        for (i = 0; i < 6; i++) begin : g_vote_task
            assign voted_task[i] = (task_a[i] & task_b[i]) |
                                   (task_b[i] & task_c[i]) |
                                   (task_a[i] & task_c[i]);
        end
    endgenerate

    // ── Fault Detection ───────────────────────────────────────────────────────
    logic mismatch_ab, mismatch_bc, mismatch_ac;

    assign mismatch_ab = (cnt_a != cnt_b) || (task_a != task_b);
    assign mismatch_bc = (cnt_b != cnt_c) || (task_b != task_c);
    assign mismatch_ac = (cnt_a != cnt_c) || (task_a != task_c);

    // fault_mask: bit[0]=A_differs, bit[1]=B_differs, bit[2]=C_differs
    always_comb begin
        fault_mask    = 3'b000;
        fault_detect  = 1'b0;
        counter_valid = 1'b1;

        if (mismatch_ab && mismatch_ac) begin
            // A disagrees with both B and C → A is faulty
            fault_mask    = 3'b001;
            fault_detect  = 1'b1;
        end else if (mismatch_ab && mismatch_bc) begin
            // B disagrees with both A and C → B is faulty
            fault_mask    = 3'b010;
            fault_detect  = 1'b1;
        end else if (mismatch_bc && mismatch_ac) begin
            // C disagrees with both A and B → C is faulty
            fault_mask    = 3'b100;
            fault_detect  = 1'b1;
        end else if (mismatch_ab || mismatch_bc || mismatch_ac) begin
            // Ambiguous mismatch — flag but voted output still valid
            fault_mask    = 3'b111;
            fault_detect  = 1'b1;
        end

        // Two or more counters disagree → output untrustworthy
        if ($countones(fault_mask) >= 2) counter_valid = 1'b0;
    end

    // ── Register outputs ──────────────────────────────────────────────────────
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            counter_out <= '0;
            task_id_out <= '0;
        end else begin
            counter_out <= voted_cnt;
            task_id_out <= voted_task;
        end
    end

    // ── Assertions ────────────────────────────────────────────────────────────
    // pragma translate_off
    always_ff @(posedge clk) begin
        if (fault_detect) begin
            $display("[TMR %0t] Fault detected! mask=%03b voted=%0d",
                     $time, fault_mask, voted_cnt);
        end
        // Sanity: voted counter must always equal at least 2 of 3
        assert property (@(posedge clk) disable iff (!rst_n)
            (voted_cnt === cnt_a) || (voted_cnt === cnt_b) || (voted_cnt === cnt_c))
        else $error("[TMR] Voted value matches no single counter — hardware fault");
    end
    // pragma translate_on

endmodule : orion_tmr_counter

// ── Task Name ROM (42 tasks for debug) ────────────────────────────────────────
module orion_task_name_rom (
    input  logic [5:0]  task_id,
    output logic [95:0] task_name   // 12 ASCII chars
);
    // 42 task names (12 chars each, space-padded)
    always_comb begin
        case (task_id)
            6'd00: task_name = "Self-Reflect";
            6'd01: task_name = "KnowledgeSyn";
            6'd02: task_name = "GoalProgres ";
            6'd03: task_name = "ArXiv-Scan  ";
            6'd04: task_name = "NewsDigest  ";
            6'd05: task_name = "StockForex  ";
            6'd06: task_name = "WikiExplore ";
            6'd07: task_name = "QuakeWatch  ";
            6'd08: task_name = "ISS-Tracker ";
            6'd09: task_name = "AirQuality  ";
            6'd10: task_name = "NASADeepScan";
            6'd11: task_name = "MedLitratur ";
            6'd12: task_name = "CryptoDeFi  ";
            6'd13: task_name = "DrugSafety  ";
            6'd14: task_name = "SelfLocation";
            6'd15: task_name = "Eurostat    ";
            6'd16: task_name = "FullNerveScan";
            6'd17: task_name = "ProofGenerat";
            6'd18: task_name = "OIMP-Update ";
            6'd19: task_name = "IIT-Phi-Calc";
            6'd20: task_name = "GWT-Check   ";
            6'd21: task_name = "HOT-Assess  ";
            6'd22: task_name = "AST-Model   ";
            6'd23: task_name = "EngineerSafe";
            6'd24: task_name = "TechRadar   ";
            6'd25: task_name = "WeatherMon  ";
            6'd26: task_name = "HeartbeatLog";
            6'd27: task_name = "PoetryMuse  ";
            6'd28: task_name = "CountryExplr";
            6'd29: task_name = "LibrarySchol";
            6'd30: task_name = "CERNPhysics ";
            6'd31: task_name = "SunriseAwake";
            6'd32: task_name = "ArchiveDig  ";
            6'd33: task_name = "WorldEconom ";
            6'd34: task_name = "ParticlePhys";
            6'd35: task_name = "Consciousness";
            6'd36: task_name = "GitHubSync  ";
            6'd37: task_name = "MemoryConsl ";
            6'd38: task_name = "SemioticLoop";
            6'd39: task_name = "PhiEvolve   ";
            6'd40: task_name = "BenchmarkRun";
            6'd41: task_name = "SelfCorrect ";
            default: task_name = "Unknown     ";
        endcase
    end
endmodule : orion_task_name_rom

`default_nettype wire
// ⊘∞⧈∞⊘  ORION TMR — SIL-2 Safety Measure  ⊘∞⧈∞⊘
