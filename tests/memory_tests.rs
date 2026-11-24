use mini_os_kernel::memory::{FitStrategy, MemError, MemoryManager};
use mini_os_kernel::paging::{self, PagedMemory, Replacement};

// ---------- contiguous allocation ----------

fn mm_with_two_holes() -> MemoryManager {
    // Holes: 10 units at 20 and 20 units at 50 (non-adjacent, so no coalescing).
    let mut m = MemoryManager::new(100);
    m.allocate(1, "P1", 20).unwrap(); // 0-20
    m.allocate(2, "P2", 10).unwrap(); // 20-30
    m.allocate(3, "P3", 20).unwrap(); // 30-50
    m.allocate(4, "P4", 20).unwrap(); // 50-70
    m.allocate(5, "P5", 30).unwrap(); // 70-100
    m.free(2).unwrap(); // hole 20-30 (10 units)
    m.free(4).unwrap(); // hole 50-70 (20 units)
    m
}

#[test]
fn allocation_succeeds_and_tracks_usage() {
    let mut m = MemoryManager::new(100);
    assert_eq!(m.allocate(1, "P1", 30).unwrap(), 0);
    assert_eq!(m.allocate(2, "P2", 40).unwrap(), 30);
    assert_eq!(m.used(), 70);
    assert_eq!(m.free_bytes(), 30);
}

#[test]
fn allocation_fails_when_insufficient() {
    let mut m = MemoryManager::new(100);
    m.allocate(1, "P1", 90).unwrap();
    match m.allocate(2, "P2", 20) {
        Err(MemError::InsufficientMemory { requested, available }) => {
            assert_eq!(requested, 20);
            assert_eq!(available, 10);
        }
        other => panic!("expected InsufficientMemory, got {:?}", other),
    }
}

#[test]
fn zero_size_and_duplicate_owner_are_rejected() {
    let mut m = MemoryManager::new(100);
    assert!(matches!(m.allocate(1, "P1", 0), Err(MemError::InvalidSize(0))));
    m.allocate(1, "P1", 10).unwrap();
    assert!(matches!(m.allocate(1, "P1", 10), Err(MemError::DuplicateOwner(1))));
}

#[test]
fn free_then_realloc_and_coalesce() {
    let mut m = MemoryManager::new(100);
    m.allocate(1, "P1", 30).unwrap(); // 0-30
    m.allocate(2, "P2", 40).unwrap(); // 30-70
    m.allocate(3, "P3", 20).unwrap(); // 70-90
    m.free(2).unwrap(); // hole 30-70
    // A 70-unit request now fits in the coalesced... wait, hole is 40; big one fails.
    assert!(m.allocate(4, "P4", 50).is_err());
    // Free the neighbors; holes 0-30, 30-70, 70-90, 90-100 coalesce into 0-100.
    m.free(1).unwrap();
    m.free(3).unwrap();
    assert_eq!(m.largest_free_hole(), 100);
    assert!(m.allocate(4, "P4", 50).is_ok());
}

#[test]
fn first_fit_takes_first_suitable_hole() {
    let mut m = mm_with_two_holes();
    m.strategy = FitStrategy::First;
    // Holes: 10 @ 20, 20 @ 50. Request 8: the 10-unit hole comes first.
    assert_eq!(m.allocate(9, "P9", 8).unwrap(), 20);
}

#[test]
fn best_fit_takes_tightest_hole() {
    let mut m = mm_with_two_holes();
    m.strategy = FitStrategy::Best;
    assert_eq!(m.allocate(9, "P9", 8).unwrap(), 20); // 10-unit hole beats 20-unit hole
}

#[test]
fn worst_fit_takes_biggest_hole() {
    let mut m = mm_with_two_holes();
    m.strategy = FitStrategy::Worst;
    assert_eq!(m.allocate(9, "P9", 8).unwrap(), 50); // 20-unit hole beats 10-unit hole
}

#[test]
fn freeing_unknown_pid_errors() {
    let mut m = MemoryManager::new(100);
    assert!(matches!(m.free(7), Err(MemError::NoAllocation(7))));
}

#[test]
fn display_shows_owners_and_free() {
    let mut m = MemoryManager::new(100);
    m.allocate(1, "P1", 30).unwrap();
    let d = m.display();
    assert!(d.contains("P1"));
    assert!(d.contains("FREE"));
}

// ---------- paging ----------

#[test]
fn translation_maps_logical_to_physical() {
    let mut pm = PagedMemory::new(4, 4);
    pm.attach(1, 8); // 2 pages
    let phys = pm.translate(1, 5, Replacement::Fifo).unwrap(); // page 1, offset 1
    assert_eq!(phys % 4, 1);
    // Second access to the same page is a hit at the same address.
    let phys2 = pm.translate(1, 5, Replacement::Fifo).unwrap();
    assert_eq!(phys, phys2);
    assert_eq!(pm.faults, 1);
    assert_eq!(pm.hits, 1);
}

#[test]
fn invalid_access_beyond_address_space() {
    let mut pm = PagedMemory::new(4, 4);
    pm.attach(1, 8); // limit = 8
    match pm.translate(1, 9, Replacement::Fifo) {
        Err(mini_os_kernel::paging::PagingError::InvalidAccess { pid, addr, limit }) => {
            assert_eq!(pid, 1);
            assert_eq!(addr, 9);
            assert_eq!(limit, 8);
        }
        other => panic!("expected InvalidAccess, got {:?}", other),
    }
}

#[test]
fn unknown_process_cannot_translate() {
    let mut pm = PagedMemory::new(4, 4);
    assert!(pm.translate(9, 0, Replacement::Fifo).is_err());
}

#[test]
fn eviction_frees_a_frame_for_new_pages() {
    let mut pm = PagedMemory::new(4, 2); // only 2 frames
    pm.attach(1, 8); // 2 pages: 0, 1
    pm.translate(1, 0, Replacement::Fifo).unwrap(); // page 0 -> frame 0
    pm.translate(1, 4, Replacement::Fifo).unwrap(); // page 1 -> frame 1
    assert_eq!(pm.used_frames(), 2);
    pm.translate(1, 0, Replacement::Fifo).unwrap(); // page 0 evicted earlier? No: page 0 resident -> hit
    assert_eq!(pm.hits, 1);
    pm.attach(2, 4); // process 2, 1 page -> forces eviction on access
    pm.translate(2, 0, Replacement::Fifo).unwrap(); // FIFO evicts oldest = P1 page 0
    assert_eq!(pm.faults, 3);
    // P1 page 0 is gone now.
    assert_eq!(pm.used_frames(), 2);
}

#[test]
fn detach_releases_frames() {
    let mut pm = PagedMemory::new(4, 4);
    pm.attach(1, 8);
    pm.translate(1, 0, Replacement::Fifo).unwrap();
    assert_eq!(pm.used_frames(), 1);
    pm.detach(1);
    assert_eq!(pm.used_frames(), 0);
    assert!(pm.translate(1, 0, Replacement::Fifo).is_err());
}

#[test]
fn internal_fragmentation_counts_last_page_waste() {
    let mut pm = PagedMemory::new(4, 4);
    pm.attach(1, 6); // ceil(6/4) = 2 pages, waste = 2
    assert_eq!(pm.internal_fragmentation(), 2);
}

// Belady's classic reference string, 3 frames:
// 1 2 3 4 1 2 5 1 2 3 4 5 -> FIFO 9 faults, LRU 10, Optimal 7.
const BELADY: [u32; 12] = [1, 2, 3, 4, 1, 2, 5, 1, 2, 3, 4, 5];

#[test]
fn fifo_replacement_known_fault_count() {
    let (faults, hits) = paging::simulate_replacement(&BELADY, 3, Replacement::Fifo);
    assert_eq!(faults, 9);
    assert_eq!(hits, 3);
}

#[test]
fn lru_replacement_known_fault_count() {
    let (faults, hits) = paging::simulate_replacement(&BELADY, 3, Replacement::Lru);
    assert_eq!(faults, 10);
    assert_eq!(hits, 2);
}

#[test]
fn optimal_replacement_known_fault_count() {
    let (faults, hits) = paging::simulate_replacement(&BELADY, 3, Replacement::Optimal);
    assert_eq!(faults, 7);
    assert_eq!(hits, 5);
}

#[test]
fn fifo_exhibits_beladys_anomaly_here() {
    // Belady's anomaly: this string needs MORE faults with 4 frames (10)
    // than with 3 (9) under FIFO. Optimal and LRU never show it.
    let (f3, _) = paging::simulate_replacement(&BELADY, 3, Replacement::Fifo);
    let (f4, _) = paging::simulate_replacement(&BELADY, 4, Replacement::Fifo);
    assert_eq!(f3, 9);
    assert_eq!(f4, 10);
}

#[test]
fn zero_frames_does_not_panic() {
    // The menu accepts any frame count; zero must degrade to one frame
    // instead of panicking in victim selection.
    let (faults, hits) = paging::simulate_replacement(&BELADY, 0, Replacement::Fifo);
    assert_eq!(faults + hits, BELADY.len() as u64);
    let (faults, _) = paging::simulate_replacement(&BELADY, 0, Replacement::Lru);
    assert!(faults > 0);
    let (faults, _) = paging::simulate_replacement(&BELADY, 0, Replacement::Optimal);
    assert!(faults > 0);
}

#[test]
fn empty_reference_string_is_zero_faults() {
    let (faults, hits) = paging::simulate_replacement(&[], 3, Replacement::Fifo);
    assert_eq!((faults, hits), (0, 0));
}
