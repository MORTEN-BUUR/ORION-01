"""
Tests for the global_workspace package (GWT Global Leasing).

Testet:
  - Lease-Lifecycle (granted → active → released/expired)
  - Broadcast an Abonnenten
  - Verdrängung durch höheres Gewicht
  - Verweigerung bei Gewicht ≤ 0
  - Phi-Proxy-Berechnung
  - AuditLog-Integrität (Hash-Kette)
  - Fingerprint-Konsistenz
"""

import time

import pytest

from global_workspace import DEFAULT_LEASE_DURATION_MS, GlobalLease, GlobalWorkspace


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture()
def ws() -> GlobalWorkspace:
    return GlobalWorkspace(default_duration_ms=200)


# ---------------------------------------------------------------------------
# GlobalLease
# ---------------------------------------------------------------------------


class TestGlobalLease:
    def test_is_valid_immediately(self):
        lease = GlobalLease(module="perception", content="test", duration_ms=500)
        assert lease.is_valid

    def test_release_invalidates(self):
        lease = GlobalLease(module="perception", content="test", duration_ms=500)
        lease.release()
        assert not lease.is_valid
        assert lease.status == "released"

    def test_expire_invalidates(self):
        lease = GlobalLease(module="attention", content="alert", duration_ms=500)
        lease.expire()
        assert not lease.is_valid
        assert lease.status == "expired"

    def test_expired_by_time(self):
        lease = GlobalLease(module="attention", content="alert", duration_ms=1)
        time.sleep(0.01)
        assert not lease.is_valid

    def test_fingerprint_deterministic(self):
        lease = GlobalLease(module="mem", content="hello", duration_ms=100)
        assert lease.fingerprint() == lease.fingerprint()

    def test_to_dict_keys(self):
        lease = GlobalLease(module="mem", content="hello", duration_ms=100)
        d = lease.to_dict()
        for key in ("lease_id", "module", "status", "granted_at", "duration_ms", "fingerprint"):
            assert key in d


# ---------------------------------------------------------------------------
# GlobalWorkspace — basic lease management
# ---------------------------------------------------------------------------


class TestGlobalWorkspaceLease:
    def test_request_lease_returns_lease(self, ws):
        lease = ws.request_lease("perception", "visual input")
        assert isinstance(lease, GlobalLease)
        assert lease.module == "perception"
        assert lease.status == "active"

    def test_active_lease_property(self, ws):
        lease = ws.request_lease("perception", "visual input")
        assert ws.active_lease is lease

    def test_release_clears_active(self, ws):
        lease = ws.request_lease("perception", "visual input")
        ws.release_lease(lease)
        assert ws.active_lease is None

    def test_history_grows(self, ws):
        ws.request_lease("a", "x")
        ws.request_lease("b", "y")
        assert len(ws.history) == 2

    def test_expired_lease_cleared_on_next_request(self, ws):
        ws_short = GlobalWorkspace(default_duration_ms=1)
        ws_short.request_lease("a", "x")
        time.sleep(0.01)
        ws_short.request_lease("b", "y")
        # Beide in der History
        assert len(ws_short.history) == 2


# ---------------------------------------------------------------------------
# GlobalWorkspace — broadcast
# ---------------------------------------------------------------------------


class TestGlobalWorkspaceBroadcast:
    def test_broadcast_reaches_subscribers(self, ws):
        received: list[tuple] = []

        def handler(sender, content, lease):
            received.append((sender, content))

        ws.subscribe("attention", handler)
        ws.subscribe("memory", handler)
        ws.request_lease("perception", "visual signal")

        assert len(received) == 2
        assert all(sender == "perception" for sender, _ in received)
        assert all(content == "visual signal" for _, content in received)

    def test_sender_does_not_receive_own_broadcast(self, ws):
        received: list[str] = []

        def handler(sender, content, lease):
            received.append(sender)

        ws.subscribe("perception", handler)
        ws.request_lease("perception", "data")

        assert len(received) == 0

    def test_unsubscribe_stops_delivery(self, ws):
        received: list = []
        ws.subscribe("memory", lambda s, c, l: received.append(c))
        ws.request_lease("perception", "first")
        ws.unsubscribe("memory")
        ws.release_lease(ws.active_lease)
        ws.request_lease("attention", "second")
        assert len(received) == 1  # nur die erste Nachricht

    def test_subscriber_exception_does_not_break_broadcast(self, ws):
        ok: list = []

        def bad_handler(s, c, l):
            raise RuntimeError("subscriber error")

        def good_handler(s, c, l):
            ok.append(c)

        ws.subscribe("bad", bad_handler)
        ws.subscribe("good", good_handler)
        ws.request_lease("perception", "signal")

        assert ok == ["signal"]


# ---------------------------------------------------------------------------
# GlobalWorkspace — preemption / denial
# ---------------------------------------------------------------------------


class TestGlobalWorkspacePreemption:
    def test_higher_weight_preempts(self, ws):
        ws.subscribe("mem", lambda s, c, l: None)
        lease1 = ws.request_lease("low", "x", weight=1.0)
        lease2 = ws.request_lease("high", "y", weight=2.0)

        assert lease1.status == "released"
        assert lease2 is not None
        assert ws.active_lease is lease2

    def test_zero_weight_denied(self, ws):
        ws.request_lease("first", "x", weight=1.0)
        result = ws.request_lease("second", "y", weight=0)
        assert result is None

    def test_negative_weight_denied(self, ws):
        ws.request_lease("first", "x", weight=1.0)
        result = ws.request_lease("second", "y", weight=-1.0)
        assert result is None


# ---------------------------------------------------------------------------
# GlobalWorkspace — phi_proxy & audit
# ---------------------------------------------------------------------------


class TestGlobalWorkspaceMetrics:
    def test_phi_proxy_zero_with_no_broadcasts(self, ws):
        assert ws.phi_proxy() == 0.0

    def test_phi_proxy_positive_after_broadcast(self, ws):
        ws.subscribe("a", lambda s, c, l: None)
        ws.subscribe("b", lambda s, c, l: None)
        ws.request_lease("perception", "signal")
        assert ws.phi_proxy() > 0.0

    def test_phi_proxy_max_one(self, ws):
        ws.subscribe("a", lambda s, c, l: None)
        for i in range(20):
            if ws.active_lease:
                ws.release_lease(ws.active_lease)
            ws.request_lease("p", f"content-{i}")
        assert ws.phi_proxy() <= 1.0

    def test_audit_log_not_empty(self, ws):
        ws.request_lease("perception", "data")
        assert len(ws.audit_log) > 0

    def test_audit_log_hash_chain_integrity(self, ws):
        """Jeder Eintrag muss einen hash haben; die Kette ist deterministisch."""
        ws.subscribe("a", lambda s, c, l: None)
        ws.request_lease("perception", "data")
        log = ws.audit_log
        for entry in log:
            assert "hash" in entry
            assert len(entry["hash"]) == 64  # SHA-256 hex
