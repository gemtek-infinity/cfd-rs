//! Edge address pool management and two-region failover (CDC-022).
//!
//! Matches the Go baseline in `edgediscovery/allregions/`:
//! - `address.go` → [`AddrSet`]
//! - `usedby.go` → [`UsedBy`]
//! - `region.go` → [`Region`]
//! - `regions.go` → [`Regions`]
//!
//! This module owns the data structures only. The actual SRV-based
//! resolution lives in the runtime (`cfdrs-bin`) transport layer.

use std::time::{Duration, Instant};

use crate::protocol::{ConfigIPVersion, EdgeAddr, EdgeIPVersion, REGION_FAILOVER_TIMEOUT_SECS};

// ---------------------------------------------------------------------------
// UsedBy (usedby.go)
// ---------------------------------------------------------------------------

/// Connection assignment state for an edge address.
///
/// Matches Go's `UsedBy` struct in `edgediscovery/allregions/usedby.go`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsedBy {
    /// Address is not assigned to any connection.
    Unused,
    /// Address is assigned to the connection with the given index.
    InUse(usize),
}

impl UsedBy {
    fn is_used(self) -> bool {
        matches!(self, UsedBy::InUse(_))
    }
}

// ---------------------------------------------------------------------------
// AddrSet (address.go)
// ---------------------------------------------------------------------------

/// A set of edge addresses with per-address connection assignment tracking.
///
/// Matches Go's `AddrSet` in `edgediscovery/allregions/address.go`.
/// Go uses `map[*EdgeAddr]UsedBy`; we use `Vec<(EdgeAddr, UsedBy)>` since
/// `EdgeAddr` does not need pointer identity — equality is by value.
#[derive(Debug, Clone)]
pub struct AddrSet {
    entries: Vec<(EdgeAddr, UsedBy)>,
}

impl AddrSet {
    /// Create an empty address set.
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Create an address set from a list of addresses, all initially unused.
    pub fn from_addrs(addrs: &[EdgeAddr]) -> Self {
        Self {
            entries: addrs.iter().map(|a| (a.clone(), UsedBy::Unused)).collect(),
        }
    }

    /// Returns the number of addresses in this set.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the set contains no addresses.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Find the address used by the given connection.
    ///
    /// Returns `None` if the connection isn't using any address in this set.
    pub fn addr_used_by(&self, conn_id: usize) -> Option<&EdgeAddr> {
        self.entries
            .iter()
            .find(|(_, used)| *used == UsedBy::InUse(conn_id))
            .map(|(addr, _)| addr)
    }

    /// Count how many addresses are not currently in use.
    pub fn available_addrs(&self) -> usize {
        self.entries.iter().filter(|(_, u)| !u.is_used()).count()
    }

    /// Return a reference to an unused address, excluding `excluding` if given.
    ///
    /// Returns `None` if all addresses are in use.
    pub fn get_unused(&self, excluding: Option<&EdgeAddr>) -> Option<&EdgeAddr> {
        self.entries
            .iter()
            .find(|(addr, used)| !used.is_used() && (excluding != Some(addr)))
            .map(|(addr, _)| addr)
    }

    /// Mark an address as in-use by the given connection.
    pub fn use_addr(&mut self, target: &EdgeAddr, conn_id: usize) {
        if let Some((_, used)) = self.entries.iter_mut().find(|(a, _)| a == target) {
            *used = UsedBy::InUse(conn_id);
        }
    }

    /// Return an arbitrary address from the set (first entry).
    pub fn get_any(&self) -> Option<&EdgeAddr> {
        self.entries.first().map(|(addr, _)| addr)
    }

    /// Release an address so other connections can use it.
    ///
    /// Returns `true` if the address was found in this set.
    pub fn give_back(&mut self, target: &EdgeAddr) -> bool {
        if let Some((_, used)) = self.entries.iter_mut().find(|(a, _)| a == target) {
            *used = UsedBy::Unused;
            true
        } else {
            false
        }
    }
}

impl Default for AddrSet {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Region (region.go)
// ---------------------------------------------------------------------------

/// A single edge region with primary/secondary address pools and IPv6→IPv4
/// failover.
///
/// Matches Go's `Region` struct in `edgediscovery/allregions/region.go`.
///
/// Primary pool is the system-preferred IP version (typically IPv6).
/// When a connectivity error occurs on IPv6, the region falls back to
/// the secondary pool (IPv4) for `REGION_FAILOVER_TIMEOUT_SECS` before
/// retrying primary.
#[derive(Debug, Clone)]
pub struct Region {
    primary_is_active: bool,
    primary: AddrSet,
    secondary: AddrSet,
    primary_timeout: Option<Instant>,
    timeout_duration: Duration,
}

impl Region {
    /// Create a region from a list of resolved edge addresses, partitioned
    /// by IP version.
    ///
    /// Matches Go's `NewRegion(addrs, overrideIPVersion)`.
    pub fn new(addrs: &[EdgeAddr], override_ip_version: ConfigIPVersion) -> Self {
        let mut v4 = Vec::new();
        let mut v6 = Vec::new();

        // First address determines system preference.
        let system_preference = addrs.first().map(|a| a.ip_version).unwrap_or(EdgeIPVersion::V6);

        for addr in addrs {
            match addr.ip_version {
                EdgeIPVersion::V4 => v4.push(addr.clone()),
                EdgeIPVersion::V6 => v6.push(addr.clone()),
            }
        }

        let (mut primary, mut secondary) = match system_preference {
            EdgeIPVersion::V4 => (AddrSet::from_addrs(&v4), AddrSet::from_addrs(&v6)),
            EdgeIPVersion::V6 => (AddrSet::from_addrs(&v6), AddrSet::from_addrs(&v4)),
        };

        match override_ip_version {
            ConfigIPVersion::IPv4Only => {
                primary = AddrSet::from_addrs(&v4);
                secondary = AddrSet::new();
            }
            ConfigIPVersion::IPv6Only => {
                primary = AddrSet::from_addrs(&v6);
                secondary = AddrSet::new();
            }
            ConfigIPVersion::Auto => {}
        }

        Self {
            primary_is_active: true,
            primary,
            secondary,
            primary_timeout: None,
            timeout_duration: Duration::from_secs(REGION_FAILOVER_TIMEOUT_SECS),
        }
    }

    /// Find the address used by the given connection.
    pub fn addr_used_by(&self, conn_id: usize) -> Option<&EdgeAddr> {
        self.primary
            .addr_used_by(conn_id)
            .or_else(|| self.secondary.addr_used_by(conn_id))
    }

    /// Count unused addresses in the active pool.
    pub fn available_addrs(&self) -> usize {
        self.active().available_addrs()
    }

    /// Assign an unused address to `conn_id`, excluding `excluding`.
    ///
    /// Returns `None` if all addresses in the active pool are in use.
    pub fn assign_any_address(&mut self, conn_id: usize, excluding: Option<&EdgeAddr>) -> Option<EdgeAddr> {
        let addr = self.active().get_unused(excluding)?.clone();
        self.active_mut().use_addr(&addr, conn_id);
        Some(addr)
    }

    /// Return an arbitrary address from the active pool.
    pub fn get_any_address(&self) -> Option<&EdgeAddr> {
        self.active().get_any()
    }

    /// Release an address and handle failover on connectivity errors.
    ///
    /// Matches Go's `Region.GiveBack(addr, hasConnectivityError)`.
    pub fn give_back(&mut self, addr: &EdgeAddr, has_connectivity_error: bool) -> bool {
        let found_primary = self.primary.give_back(addr);
        let found_secondary = if !found_primary {
            self.secondary.give_back(addr)
        } else {
            false
        };

        if !found_primary && !found_secondary {
            return false;
        }

        if !has_connectivity_error {
            return true;
        }

        // Using primary and IPv6 failed — fall back to secondary (IPv4).
        if self.primary_is_active && addr.ip_version == EdgeIPVersion::V6 && !self.secondary.is_empty() {
            self.primary_is_active = false;
            self.primary_timeout = Some(Instant::now() + self.timeout_duration);
            return true;
        }

        // Still on primary — no further action.
        if self.primary_is_active {
            return true;
        }

        // On secondary with IPv4 error — immediately retry primary.
        if addr.ip_version == EdgeIPVersion::V4 {
            self.activate_primary();
            return true;
        }

        // Check if the timeout has elapsed — retry primary if so.
        if let Some(deadline) = self.primary_timeout
            && Instant::now() >= deadline
        {
            self.activate_primary();
        }

        true
    }

    fn activate_primary(&mut self) {
        self.primary_is_active = true;
        self.primary_timeout = None;
    }

    fn active(&self) -> &AddrSet {
        if self.primary_is_active {
            &self.primary
        } else {
            &self.secondary
        }
    }

    fn active_mut(&mut self) -> &mut AddrSet {
        if self.primary_is_active {
            &mut self.primary
        } else {
            &mut self.secondary
        }
    }
}

// ---------------------------------------------------------------------------
// Regions (regions.go)
// ---------------------------------------------------------------------------

/// Two-region edge address manager for redundancy.
///
/// Matches Go's `Regions` struct in `edgediscovery/allregions/regions.go`.
/// NOT thread-safe — callers must synchronize access.
#[derive(Debug, Clone)]
pub struct Regions {
    region1: Region,
    region2: Region,
}

impl Regions {
    /// Create from pre-resolved per-region address lists.
    ///
    /// `region_addrs` must contain at least 2 entries — one per SRV CNAME
    /// target (region). Returns `Err` if fewer than 2 regions are provided.
    ///
    /// Matches Go's `ResolveEdge` constructor.
    pub fn from_resolved(
        region_addrs: &[Vec<EdgeAddr>],
        override_ip_version: ConfigIPVersion,
    ) -> Result<Self, String> {
        if region_addrs.len() < 2 {
            return Err(format!(
                "expected at least 2 Cloudflare edge regions, but SRV only returned {}",
                region_addrs.len()
            ));
        }

        Ok(Self {
            region1: Region::new(&region_addrs[0], override_ip_version),
            region2: Region::new(&region_addrs[1], override_ip_version),
        })
    }

    /// Create from a flat list of addresses, distributing evenly across two
    /// regions (round-robin).
    ///
    /// Matches Go's `NewNoResolve`.
    pub fn from_flat(addrs: &[EdgeAddr], override_ip_version: ConfigIPVersion) -> Self {
        let mut r1 = Vec::new();
        let mut r2 = Vec::new();

        for (i, addr) in addrs.iter().enumerate() {
            if i % 2 == 0 {
                r1.push(addr.clone());
            } else {
                r2.push(addr.clone());
            }
        }

        Self {
            region1: Region::new(&r1, override_ip_version),
            region2: Region::new(&r2, override_ip_version),
        }
    }

    /// Return an arbitrary address from the larger region.
    pub fn get_any_address(&self) -> Option<&EdgeAddr> {
        self.region1
            .get_any_address()
            .or_else(|| self.region2.get_any_address())
    }

    /// Find the address used by the given connection.
    pub fn addr_used_by(&self, conn_id: usize) -> Option<&EdgeAddr> {
        self.region1
            .addr_used_by(conn_id)
            .or_else(|| self.region2.addr_used_by(conn_id))
    }

    /// Get an unused address, preferring the region with more available
    /// addresses. Assigns the address to `conn_id`.
    ///
    /// Matches Go's `GetUnusedAddr(excluding, connID)`.
    pub fn get_unused_addr(&mut self, excluding: Option<&EdgeAddr>, conn_id: usize) -> Option<EdgeAddr> {
        let avail1 = self.region1.available_addrs();
        let avail2 = self.region2.available_addrs();

        if avail1 >= avail2 {
            self.region1
                .assign_any_address(conn_id, excluding)
                .or_else(|| self.region2.assign_any_address(conn_id, excluding))
        } else {
            self.region2
                .assign_any_address(conn_id, excluding)
                .or_else(|| self.region1.assign_any_address(conn_id, excluding))
        }
    }

    /// Count total unused addresses across both regions.
    pub fn available_addrs(&self) -> usize {
        self.region1.available_addrs() + self.region2.available_addrs()
    }

    /// Release an address so other connections can use it.
    pub fn give_back(&mut self, addr: &EdgeAddr, has_connectivity_error: bool) -> bool {
        if self.region1.give_back(addr, has_connectivity_error) {
            return true;
        }
        self.region2.give_back(addr, has_connectivity_error)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

    use super::*;

    fn v4_addr(last_octet: u8, port: u16) -> EdgeAddr {
        let ip = Ipv4Addr::new(198, 41, 200, last_octet);
        EdgeAddr {
            tcp: SocketAddr::new(ip.into(), port),
            udp: SocketAddr::new(ip.into(), port),
            ip_version: EdgeIPVersion::V4,
        }
    }

    fn v6_addr(segment: u16, port: u16) -> EdgeAddr {
        let ip = Ipv6Addr::new(0x2606, 0x4700, 0, 0, 0, 0, 0, segment);
        EdgeAddr {
            tcp: SocketAddr::new(ip.into(), port),
            udp: SocketAddr::new(ip.into(), port),
            ip_version: EdgeIPVersion::V6,
        }
    }

    // -----------------------------------------------------------------------
    // UsedBy
    // -----------------------------------------------------------------------

    #[test]
    fn unused_is_not_used() {
        assert!(!UsedBy::Unused.is_used());
    }

    #[test]
    fn in_use_is_used() {
        assert!(UsedBy::InUse(0).is_used());
        assert!(UsedBy::InUse(42).is_used());
    }

    // -----------------------------------------------------------------------
    // AddrSet
    // -----------------------------------------------------------------------

    #[test]
    fn empty_addr_set() {
        let set = AddrSet::new();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
        assert_eq!(set.available_addrs(), 0);
        assert!(set.get_any().is_none());
        assert!(set.get_unused(None).is_none());
        assert!(set.addr_used_by(0).is_none());
    }

    #[test]
    fn addr_set_from_addrs_all_unused() {
        let addrs = vec![v4_addr(1, 7844), v4_addr(2, 7844)];
        let set = AddrSet::from_addrs(&addrs);
        assert_eq!(set.len(), 2);
        assert_eq!(set.available_addrs(), 2);
    }

    #[test]
    fn use_and_give_back() {
        let addrs = vec![v4_addr(1, 7844), v4_addr(2, 7844)];
        let mut set = AddrSet::from_addrs(&addrs);

        set.use_addr(&addrs[0], 0);
        assert_eq!(set.available_addrs(), 1);
        assert_eq!(set.addr_used_by(0), Some(&addrs[0]));

        assert!(set.give_back(&addrs[0]));
        assert_eq!(set.available_addrs(), 2);
        assert!(set.addr_used_by(0).is_none());
    }

    #[test]
    fn give_back_unknown_addr_returns_false() {
        let mut set = AddrSet::from_addrs(&[v4_addr(1, 7844)]);
        assert!(!set.give_back(&v4_addr(99, 7844)));
    }

    #[test]
    fn get_unused_excludes_specified_addr() {
        let addrs = vec![v4_addr(1, 7844), v4_addr(2, 7844)];
        let set = AddrSet::from_addrs(&addrs);
        let got = set.get_unused(Some(&addrs[0]));
        assert_eq!(got, Some(&addrs[1]));
    }

    #[test]
    fn get_unused_returns_none_when_all_excluded_or_used() {
        let addrs = vec![v4_addr(1, 7844)];
        let set = AddrSet::from_addrs(&addrs);
        assert!(set.get_unused(Some(&addrs[0])).is_none());
    }

    // -----------------------------------------------------------------------
    // Region
    // -----------------------------------------------------------------------

    #[test]
    fn region_new_partitions_by_ip_version() {
        let addrs = vec![v6_addr(1, 7844), v4_addr(1, 7844), v6_addr(2, 7844)];
        let region = Region::new(&addrs, ConfigIPVersion::Auto);

        // First addr is v6, so primary=v6 (2 addrs), secondary=v4 (1 addr).
        // Active pool is primary (v6), so 2 available.
        assert_eq!(region.available_addrs(), 2);
    }

    #[test]
    fn region_ipv4_only_override() {
        let addrs = vec![v6_addr(1, 7844), v4_addr(1, 7844), v4_addr(2, 7844)];
        let region = Region::new(&addrs, ConfigIPVersion::IPv4Only);

        // Override: primary=v4 (2 addrs), secondary=empty.
        assert_eq!(region.available_addrs(), 2);
    }

    #[test]
    fn region_ipv6_only_override() {
        let addrs = vec![v4_addr(1, 7844), v6_addr(1, 7844)];
        let region = Region::new(&addrs, ConfigIPVersion::IPv6Only);

        // Override: primary=v6 (1 addr), secondary=empty.
        assert_eq!(region.available_addrs(), 1);
    }

    #[test]
    fn region_assign_and_lookup() {
        let addrs = vec![v4_addr(1, 7844), v4_addr(2, 7844)];
        let mut region = Region::new(&addrs, ConfigIPVersion::Auto);

        let assigned = region.assign_any_address(0, None);
        assert!(assigned.is_some());
        assert!(region.addr_used_by(0).is_some());
        assert_eq!(region.available_addrs(), 1);
    }

    #[test]
    fn region_assign_returns_none_when_exhausted() {
        let addrs = vec![v4_addr(1, 7844)];
        let mut region = Region::new(&addrs, ConfigIPVersion::Auto);

        region.assign_any_address(0, None);
        assert!(region.assign_any_address(1, None).is_none());
    }

    #[test]
    fn region_give_back_no_error_stays_on_primary() {
        let addrs = vec![v6_addr(1, 7844), v4_addr(1, 7844)];
        let mut region = Region::new(&addrs, ConfigIPVersion::Auto);

        let assigned = region.assign_any_address(0, None).expect("should assign");
        region.give_back(&assigned, false);

        // No connectivity error: stays on primary (v6).
        assert_eq!(region.available_addrs(), 1); // v6 pool size
    }

    #[test]
    fn region_give_back_v6_error_falls_back_to_secondary() {
        let addrs = vec![v6_addr(1, 7844), v4_addr(1, 7844)];
        let mut region = Region::new(&addrs, ConfigIPVersion::Auto);

        let v6 = region.assign_any_address(0, None).expect("should assign v6");
        assert_eq!(v6.ip_version, EdgeIPVersion::V6);

        region.give_back(&v6, true);

        // Connectivity error on v6: should fall back to secondary (v4).
        // Active pool is now secondary with 1 v4 addr.
        assert_eq!(region.available_addrs(), 1);

        // The available address should be v4.
        let next = region.get_any_address().expect("should have v4");
        assert_eq!(next.ip_version, EdgeIPVersion::V4);
    }

    #[test]
    fn region_give_back_v4_error_returns_to_primary() {
        let addrs = vec![v6_addr(1, 7844), v4_addr(1, 7844)];
        let mut region = Region::new(&addrs, ConfigIPVersion::Auto);

        // First: fall back to secondary by returning v6 with error.
        let v6 = region.assign_any_address(0, None).expect("should assign v6");
        region.give_back(&v6, true);

        // Now on secondary (v4). Assign and return v4 with error.
        let v4 = region.assign_any_address(1, None).expect("should assign v4");
        assert_eq!(v4.ip_version, EdgeIPVersion::V4);
        region.give_back(&v4, true);

        // Should immediately return to primary (v6).
        let next = region.get_any_address().expect("should have v6");
        assert_eq!(next.ip_version, EdgeIPVersion::V6);
    }

    // -----------------------------------------------------------------------
    // Regions
    // -----------------------------------------------------------------------

    #[test]
    fn regions_from_resolved_requires_two_regions() {
        let one_region = vec![vec![v4_addr(1, 7844)]];
        assert!(Regions::from_resolved(&one_region, ConfigIPVersion::Auto).is_err());
    }

    #[test]
    fn regions_from_resolved_two_regions() {
        let region_addrs = vec![
            vec![v4_addr(1, 7844), v4_addr(2, 7844)],
            vec![v4_addr(3, 7844), v4_addr(4, 7844)],
        ];
        let regions = Regions::from_resolved(&region_addrs, ConfigIPVersion::Auto).expect("should succeed");
        assert_eq!(regions.available_addrs(), 4);
    }

    #[test]
    fn regions_from_flat_distributes_round_robin() {
        let addrs = vec![
            v4_addr(1, 7844),
            v4_addr(2, 7844),
            v4_addr(3, 7844),
            v4_addr(4, 7844),
        ];
        let regions = Regions::from_flat(&addrs, ConfigIPVersion::Auto);
        // r1 gets indices 0,2; r2 gets indices 1,3.
        assert_eq!(regions.available_addrs(), 4);
    }

    #[test]
    fn regions_get_unused_balances_across_regions() {
        let region_addrs = vec![
            vec![v4_addr(1, 7844), v4_addr(2, 7844)],
            vec![v4_addr(3, 7844), v4_addr(4, 7844)],
        ];
        let mut regions =
            Regions::from_resolved(&region_addrs, ConfigIPVersion::Auto).expect("should succeed");

        let a1 = regions.get_unused_addr(None, 0);
        assert!(a1.is_some());

        let a2 = regions.get_unused_addr(None, 1);
        assert!(a2.is_some());

        // After assigning one from each region, 2 remain.
        assert_eq!(regions.available_addrs(), 2);
    }

    #[test]
    fn regions_addr_used_by_finds_across_regions() {
        let region_addrs = vec![vec![v4_addr(1, 7844)], vec![v4_addr(2, 7844)]];
        let mut regions =
            Regions::from_resolved(&region_addrs, ConfigIPVersion::Auto).expect("should succeed");

        regions.get_unused_addr(None, 0);
        assert!(regions.addr_used_by(0).is_some());
    }

    #[test]
    fn regions_give_back_across_regions() {
        let region_addrs = vec![vec![v4_addr(1, 7844)], vec![v4_addr(2, 7844)]];
        let mut regions =
            Regions::from_resolved(&region_addrs, ConfigIPVersion::Auto).expect("should succeed");

        let addr = regions.get_unused_addr(None, 0).expect("should assign");
        assert_eq!(regions.available_addrs(), 1);

        assert!(regions.give_back(&addr, false));
        assert_eq!(regions.available_addrs(), 2);
    }

    #[test]
    fn regions_give_back_unknown_returns_false() {
        let region_addrs = vec![vec![v4_addr(1, 7844)], vec![v4_addr(2, 7844)]];
        let mut regions =
            Regions::from_resolved(&region_addrs, ConfigIPVersion::Auto).expect("should succeed");

        assert!(!regions.give_back(&v4_addr(99, 7844), false));
    }

    #[test]
    fn regions_get_any_address() {
        let region_addrs = vec![vec![v4_addr(1, 7844)], vec![v4_addr(2, 7844)]];
        let regions = Regions::from_resolved(&region_addrs, ConfigIPVersion::Auto).expect("should succeed");
        assert!(regions.get_any_address().is_some());
    }
}
