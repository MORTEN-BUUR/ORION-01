"""
global_workspace — Global Workspace Leasing für ORION (GWT / Baars 1988).

Kernklassen:
    GlobalLease      — Zeit-limitiertes Broadcast-Token
    GlobalWorkspace  — Lease-Manager + Broadcast-Bus
"""

from global_workspace.lease import GlobalLease
from global_workspace.workspace import DEFAULT_LEASE_DURATION_MS, GlobalWorkspace

__all__ = ["GlobalLease", "GlobalWorkspace", "DEFAULT_LEASE_DURATION_MS"]
__version__ = "0.1.0"
