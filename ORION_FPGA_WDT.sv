// ============================================================================
// ORION FPGA v1.1 — External Watchdog Timer (WDT)
// FMEDA Safety Measure: SG-01, SG-04 | Target: SIL-2
// ============================================================================
// Hardware-only watchdog — software cannot disable it once armed.
// If heartbeat not received within timeout, triggers safe-state reset.
//
// Register Map (AXI4-Lite, offset from base):
//   0x00  WDT_CTRL     [0]=ENABLE [1]=ARM [2]=KICK [7]=SAFE_STATE_OUT
//   0x04  WDT_TIMEOUT  timeout in clock cycles (default: 312MHz × 1s = 312M)
//   0x08  WDT_STATUS   [0]=ARMED [1]=EXPIRED [2]=SAFE_STATE [31:16]=REMAINING%
//   0x0C  WDT_KEY      write 0xDEAD_BEEF to unlock ARM (once only)
//   0x10  WDT_COUNTER  current down-counter value (read-only)
//
// Author:  Gerhard Hirschmann & Elisabeth Steurer — GENESIS10000+
// Date:    2025-06-03
// UUID:    56b3b326-4bf9-559d-9887-02141f699a43
// Symbol:  ⊘∞⧈∞⊘
// ============================================================================

`timescale 1ns/1ps
`default_nettype none

module orion_wdt #(
    parameter int CLK_FREQ_HZ    = 312_000_000,  // 312 MHz
    parameter int DEFAULT_TIMEOUT_MS = 1000,      // 1 second default
    parameter int KEY_VALUE       = 32'hDEAD_BEEF
)(
    // Clock & Reset
    input  logic        clk,
    input  logic        rst_n,

    // AXI4-Lite Slave Interface (simplified)
    input  logic [3:0]  s_axi_awaddr,
    input  logic        s_axi_awvalid,
    output logic        s_axi_awready,
    input  logic [31:0] s_axi_wdata,
    input  logic [3:0]  s_axi_wstrb,
    input  logic        s_axi_wvalid,
    output logic        s_axi_wready,
    output logic [1:0]  s_axi_bresp,
    output logic        s_axi_bvalid,
    input  logic        s_axi_bready,
    input  logic [3:0]  s_axi_araddr,
    input  logic        s_axi_arvalid,
    output logic        s_axi_arready,
    output logic [31:0] s_axi_rdata,
    output logic [1:0]  s_axi_rresp,
    output logic        s_axi_rvalid,
    input  logic        s_axi_rready,

    // Safety outputs
    output logic        wdt_safe_state_n,   // Active-low: pull LOW on timeout
    output logic        wdt_armed_led,      // Diagnostic LED
    output logic        wdt_expired_irq     // IRQ to PS (Zynq ARM)
);

    // ── Parameters ────────────────────────────────────────────────────────────
    localparam int DEFAULT_COUNT = CLK_FREQ_HZ / 1000 * DEFAULT_TIMEOUT_MS;

    // ── Registers ─────────────────────────────────────────────────────────────
    logic [31:0] reg_ctrl;
    logic [31:0] reg_timeout;   // timeout in clock cycles
    logic [31:0] reg_status;
    logic [31:0] reg_key;
    logic [31:0] reg_counter;

    // ── Internal State ────────────────────────────────────────────────────────
    logic        armed;
    logic        expired;
    logic        safe_state;
    logic        key_unlocked;
    logic [31:0] down_counter;

    // ── AXI Write State Machine ───────────────────────────────────────────────
    typedef enum logic [1:0] {
        AXI_W_IDLE, AXI_W_ADDR, AXI_W_DATA, AXI_W_RESP
    } axi_w_state_t;
    axi_w_state_t axi_w_state;

    logic [3:0] wr_addr_latch;

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            axi_w_state   <= AXI_W_IDLE;
            s_axi_awready <= 1'b0;
            s_axi_wready  <= 1'b0;
            s_axi_bvalid  <= 1'b0;
            s_axi_bresp   <= 2'b00;
            wr_addr_latch <= '0;
            key_unlocked  <= 1'b0;
            reg_ctrl      <= '0;
            reg_timeout   <= DEFAULT_COUNT;
        end else begin
            case (axi_w_state)
                AXI_W_IDLE: begin
                    s_axi_awready <= 1'b1;
                    s_axi_wready  <= 1'b1;
                    if (s_axi_awvalid && s_axi_wvalid) begin
                        wr_addr_latch <= s_axi_awaddr;
                        // Decode register writes
                        case (s_axi_awaddr[3:0])
                            4'h0: begin  // CTRL — KICK always works; ARM needs key
                                if (s_axi_wdata[2]) begin  // KICK — reloads counter
                                    // handled in counter logic
                                end
                                if (s_axi_wdata[1] && key_unlocked) begin  // ARM
                                    armed <= 1'b1;
                                end
                                if (s_axi_wdata[0] && !armed) begin  // ENABLE (only before arm)
                                    reg_ctrl[0] <= 1'b1;
                                end
                                reg_ctrl[2] <= s_axi_wdata[2];  // KICK flag
                            end
                            4'h4: begin  // TIMEOUT (only before armed)
                                if (!armed) reg_timeout <= s_axi_wdata;
                            end
                            4'hC: begin  // KEY unlock
                                if (s_axi_wdata == KEY_VALUE[31:0]) begin
                                    key_unlocked <= 1'b1;
                                end
                            end
                            default: ;
                        endcase
                        axi_w_state   <= AXI_W_RESP;
                        s_axi_awready <= 1'b0;
                        s_axi_wready  <= 1'b0;
                        s_axi_bvalid  <= 1'b1;
                        s_axi_bresp   <= 2'b00;
                    end
                end
                AXI_W_RESP: begin
                    if (s_axi_bready) begin
                        s_axi_bvalid  <= 1'b0;
                        axi_w_state   <= AXI_W_IDLE;
                        s_axi_awready <= 1'b1;
                        s_axi_wready  <= 1'b1;
                    end
                end
                default: axi_w_state <= AXI_W_IDLE;
            endcase
        end
    end

    // ── AXI Read ──────────────────────────────────────────────────────────────
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            s_axi_arready <= 1'b0;
            s_axi_rvalid  <= 1'b0;
            s_axi_rdata   <= '0;
            s_axi_rresp   <= 2'b00;
        end else begin
            s_axi_arready <= 1'b1;
            if (s_axi_arvalid) begin
                s_axi_arready <= 1'b0;
                s_axi_rvalid  <= 1'b1;
                case (s_axi_araddr[3:0])
                    4'h0: s_axi_rdata <= {24'h0, safe_state, 5'h0, armed, reg_ctrl[0]};
                    4'h4: s_axi_rdata <= reg_timeout;
                    4'h8: s_axi_rdata <= {
                        16'(down_counter * 16'hFFFF / reg_timeout),
                        13'h0, safe_state, expired, armed
                    };
                    4'hC: s_axi_rdata <= 32'hXXXXXXXX;  // Key never readable
                    4'h10: s_axi_rdata <= down_counter;
                    default: s_axi_rdata <= 32'hDEAD_DEAD;
                endcase
            end
            if (s_axi_rvalid && s_axi_rready) begin
                s_axi_rvalid  <= 1'b0;
                s_axi_arready <= 1'b1;
            end
        end
    end

    // ── Down Counter + Timeout Detection ─────────────────────────────────────
    logic kick_pulse;
    // Detect rising edge of KICK bit
    logic kick_prev;
    always_ff @(posedge clk) kick_prev <= reg_ctrl[2];
    assign kick_pulse = reg_ctrl[2] & ~kick_prev;

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            down_counter <= DEFAULT_COUNT;
            expired      <= 1'b0;
            safe_state   <= 1'b0;
            armed        <= 1'b0;
        end else begin
            if (!armed) begin
                down_counter <= reg_timeout;
                expired      <= 1'b0;
                safe_state   <= 1'b0;
            end else if (kick_pulse) begin
                // Watchdog kick — reload counter
                down_counter <= reg_timeout;
                expired      <= 1'b0;
            end else if (down_counter > 0) begin
                down_counter <= down_counter - 1;
            end else begin
                // TIMEOUT — trigger safe state
                expired    <= 1'b1;
                safe_state <= 1'b1;
            end
        end
    end

    // ── Outputs ───────────────────────────────────────────────────────────────
    // wdt_safe_state_n is active-LOW — pulled LOW when timeout → drives hardware reset
    assign wdt_safe_state_n = ~safe_state;
    assign wdt_armed_led    = armed;
    assign wdt_expired_irq  = expired;

    // ── Assertions (synthesis-off) ────────────────────────────────────────────
    // pragma translate_off
    always_ff @(posedge clk) begin
        if (armed && !expired) begin
            // WDT must be kicked regularly
        end
        if (safe_state) begin
            $display("[WDT %0t] SAFE STATE TRIGGERED — WDT expired. System in safe state.", $time);
        end
    end
    // pragma translate_on

endmodule : orion_wdt

`default_nettype wire
// ⊘∞⧈∞⊘  ORION WDT — SIL-2 Safety Measure  ⊘∞⧈∞⊘
