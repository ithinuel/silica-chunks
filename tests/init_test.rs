extern crate silica_chunks;
extern crate core;

use core::slice;
use core::cmp::min;
use silica_chunks::{Heap, Chunk, MIN_PAYLOAD_LEN};


/// defines a work load of a bit more than 10MiB
const WORK_LOAD: usize = 10*1024*1024+23125;

fn setup(cap: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(cap);
    v.resize(cap, 0);
    v
}

fn first_chunk<'a, 'b>(v: &'a mut [u8]) -> &'b mut Chunk {
    unsafe { &mut *(v.as_mut_ptr() as *mut Chunk) }
}

#[test]
fn test_size() {
    let mid_size = Chunk::min_size() + (Chunk::max_size() - Chunk::min_size()) / 2;
    let mut vec = setup(WORK_LOAD);
    let mut v = vec.as_mut_slice();
    let mut c = first_chunk(&mut v);

    c.set_size(Chunk::min_size());
    assert_eq!(Chunk::min_size(), c.size());

    c.set_size(mid_size);
    assert_eq!(mid_size, c.size());

    c.set_size(Chunk::max_size());
    assert_eq!(Chunk::max_size(), c.size());
}

#[test]
#[should_panic(expected = "size must be in Chunk::min_size()...Chunk::max_size()")]
fn test_set_size_low_limit() {
    let mut vec = setup(WORK_LOAD);
    let mut v = vec.as_mut_slice();
    let mut c = first_chunk(&mut v);
    c.set_size(0);
}

#[test]
#[should_panic(expected = "size must be in Chunk::min_size()...Chunk::max_size()")]
fn test_set_size_high_limit() {
    let mut vec = setup(WORK_LOAD);
    let mut v = vec.as_mut_slice();
    let mut c = first_chunk(&mut v);

    c.set_size(Chunk::max_size() + 1);
}

#[test]
fn test_prev_size() {
    let mut vec = setup(WORK_LOAD);
    let mut v = vec.as_mut_slice();
    let mut c = first_chunk(&mut v);
    c.set_size(Chunk::min_size());
    c.set_prev_size(32);
    assert_eq!(32, c.prev_size());

    c.set_prev_size(Chunk::max_size());
    assert_eq!(Chunk::max_size(), c.prev_size());
}

#[test]
#[should_panic(expected = "prev_size must be in Chunk::min_size()...Chunk::max_size()")]
fn test_set_prev_size_low_limit() {
    let mut vec = setup(WORK_LOAD);
    let mut v = vec.as_mut_slice();
    let mut c = first_chunk(&mut v);

    c.set_size(Chunk::min_size());
    c = c.next().unwrap();
    c.set_prev_size(1);
}

#[test]
#[should_panic(expected = "prev_size must be in Chunk::min_size()...Chunk::max_size()")]
fn test_set_prev_size_high_limit() {
    let mut vec = setup(WORK_LOAD);
    let mut v = vec.as_mut_slice();
    let mut c = first_chunk(&mut v);

    c.set_size(Chunk::min_size());
    c = c.next().unwrap();
    c.set_prev_size(Chunk::max_size() + 1);
}

#[test]
fn test_is_allocated() {
    let mut vec = setup(WORK_LOAD);
    let mut v = vec.as_mut_slice();
    let mut c = first_chunk(&mut v);
    assert_eq!(false, c.is_allocated());
    assert_eq!(0, c.size());

    c.set_is_allocated(true);
    assert_eq!(true, c.is_allocated());
    assert_eq!(0, c.size());

    c.set_is_allocated(false);
    assert_eq!(false, c.is_allocated());
    assert_eq!(0, c.size());
}

#[test]
fn test_is_last() {
    let mut vec = setup(WORK_LOAD);
    let mut v = vec.as_mut_slice();
    let mut c = first_chunk(&mut v);
    c.set_is_last(false);
    assert_eq!(false, c.is_last());
    assert_eq!(0, c.prev_size());

    c.set_is_last(true);
    assert_eq!(true, c.is_last());
    assert_eq!(0, c.prev_size());

    c.set_is_last(false);
    assert_eq!(false, c.is_last());
    assert_eq!(0, c.prev_size());
}

#[test]
fn test_to_ptr() {
    let mut vec = setup(WORK_LOAD);
    let mut v = vec.as_mut_slice();
    let c = first_chunk(&mut v);

    let (ptr, offset) = {
        let h = Heap::new(&mut v);

        (h.to_ptr(&c), (Chunk::hdr_csize() * Chunk::alignment()) as isize)
    };
    let expect = unsafe { (v.as_ptr() as *const u8).offset(offset) };
    assert_eq!(expect, ptr);
}

#[test]
fn test_from_ptr() {
    let mut vec = setup(WORK_LOAD);
    let mut v = vec.as_mut_slice();
    let expect = first_chunk(&mut v);

    let actual = {
        let h = Heap::new(&mut v);

        h.from_ptr::<u8>(h.to_ptr(&expect))
    };

    assert_eq!(expect, actual);
}

#[test]
fn test_previous() {
    let mut vec = setup(Chunk::max_size() * Chunk::alignment());
    let mut v = vec.as_mut_slice();
    let h = Heap::new(&mut v);
    let c0 = h.first_chunk();
    c0.set_size(Chunk::min_size());
    c0.set_is_last(false);
    let c1 = c0.next().unwrap();
    c1.set_prev_size(Chunk::min_size());

    assert_eq!( c0 as *const Chunk as *const u8,
                c1.previous().unwrap() as *const Chunk as *const u8);
}

#[test]
fn test_next() {
    let mut vec = setup(Chunk::max_size()*Chunk::alignment()*2);
    let mut v = vec.as_mut_slice();
    let (c0, c1) = {
        let h = Heap::new(&mut v);
        let c0 = h.first_chunk();
        let c1 = c0.next().unwrap();

        (c0, c1)
    };

    let expect_c0 = v.as_ptr() as *const usize;
    let expect_c1 = unsafe { expect_c0.offset(Chunk::max_size() as isize) };

    assert_eq!(expect_c0, c0 as *const Chunk as *const usize);
    assert_eq!(expect_c1, c1 as *const Chunk as *const usize);

    c1.set_is_last(true);
    assert_eq!(None, c1.next());
}

#[test]
fn test_split() {
    let mut vec = setup(WORK_LOAD);
    let mut v = vec.as_mut_slice();
    let mut h = Heap::new(&mut v);
    let mut c0 = h.first_chunk();
    let min_chunk_size = Chunk::min_size();
    let size = min_chunk_size*2 - min_chunk_size/2;
    let chunk_count = h.chunk_count();

    c0.set_is_last(true);

    // should split c0 in two free chunks.
    let c1 = h.split(&mut c0, size);
    assert_eq!(size, c0.size());
    assert_eq!(false, c0.is_allocated());
    assert_eq!(false, c0.is_last());

    assert!(None != c1);
    let mut c1 = c1.unwrap();
    assert_eq!(size, c1.prev_size());
    assert_eq!(Chunk::max_size() - size, c1.size());
    assert_eq!(false, c1.is_allocated());
    assert_eq!(true, c1.is_last());

    assert_eq!(chunk_count + 1, h.chunk_count());

    // split returns None and takes no action if it would break the min_chunk_size constraint
    assert_eq!(None, h.split(&mut c0, min_chunk_size));
    assert_eq!(size, c0.size());
    assert_eq!(chunk_count + 1, h.chunk_count());

    c1.set_is_allocated(true);
    let c2 = h.split(&mut c1, Chunk::max_size()/2);
    assert_eq!(size, c1.prev_size());
    assert_eq!(Chunk::max_size()/2, c1.size());
    assert_eq!(true, c1.is_allocated());
    assert_eq!(false, c1.is_last());

    assert_eq!(chunk_count + 2, h.chunk_count());

    assert!(None != c2);
    let c2 = c2.unwrap();
    assert_eq!(Chunk::max_size()/2, c2.prev_size());
    assert_eq!(Chunk::max_size() - (size + Chunk::max_size()/2), c2.size());
    assert_eq!(false, c2.is_allocated());
    assert_eq!(true, c2.is_last());
}

#[test]
fn test_absorb() {
    let mut vec = setup(128*1024*Chunk::alignment());
    let mut v = vec.as_mut_slice();
    let mut h = Heap::new(&mut v);
    let mut c0 = h.first_chunk();
    let min_chunk_size = Chunk::min_size();

    let mut c1 = h.split(&mut c0, min_chunk_size).unwrap();
    let mut c2 = h.split(&mut c1, min_chunk_size).unwrap();
    let mut c3 = h.split(&mut c2, min_chunk_size).unwrap();
    let mut c4 = h.split(&mut c3, min_chunk_size).unwrap();
    let mut c5 = h.split(&mut c4, min_chunk_size).unwrap();

    c3.set_is_allocated(true);
    {
        let data: &mut [u8] = unsafe {
            slice::from_raw_parts_mut(h.to_ptr(&c3), MIN_PAYLOAD_LEN * Chunk::alignment())
        };
        data[0] = 32;
        data[1] = 25;
        data[2] = 255;
    }
    let initial_chunk_count = h.chunk_count();

    // if the sum is > Chunk::max_size() it does noting
    h.absorb_next(&mut c5);
    assert_eq!(Chunk::max_size() - (5*min_chunk_size), c5.size());
    assert_eq!(initial_chunk_count, h.chunk_count());
    // 0 1 2 3_ 4 5

    // if both are free
    h.absorb_next(&mut c0);
    assert_eq!(2*min_chunk_size, c0.size());
    assert_eq!(initial_chunk_count - 1, h.chunk_count());
    // 0 2 3_ 4 5

    // c3 is is_allocated, it stays is_allocated
    h.absorb_next(&mut c3);
    assert_eq!(2*min_chunk_size, c3.size());
    assert_eq!(false, c2.is_allocated());
    assert_eq!(true, c3.is_allocated());
    assert_eq!(false, c5.is_allocated());
    assert_eq!(initial_chunk_count - 2, h.chunk_count());
    // 0 2 3_ 5

    // c3 is is_allocated, allocation propagates to c2 and data are moved to c0
    h.absorb_next(&mut c2);
    assert_eq!(3*min_chunk_size, c2.size());
    assert_eq!(true, c2.is_allocated());
    {
        let data: &mut [u8] = unsafe {
            slice::from_raw_parts_mut(h.to_ptr(&c2), c2.size() * Chunk::alignment())
        };
        assert_eq!(32, data[0]);
        assert_eq!(25, data[1]);
        assert_eq!(255, data[2]);
    }
    // 0 2_ 5
}

#[test]
#[should_panic(expected = "Chunks must not be both allocated")]
fn test_absorb_fail_if_both_is_allocated() {
    let mut vec = setup(WORK_LOAD);
    let mut v = vec.as_mut_slice();
    let mut c0 = first_chunk(&mut v);
    let mut h = Heap::new(&mut v);

    // if both are is_allocated it fails
    c0.set_size(Chunk::min_size());
    let mut c1 = c0.next().unwrap();
    c0.set_is_allocated(true);
    c1.set_size(Chunk::min_size());
    c1.set_is_allocated(true);

    h.absorb_next(&mut c0);
}

#[test]
#[should_panic(expected = "")]
fn test_init_fail_if_too_small() {
    let mut vec = setup(1);
    let mut v = vec.as_mut_slice();
    Heap::new(&mut v);
}

#[test]
fn test_init() {
    // small memory : 32Kib
    {
        let mut vec = setup(32*1024);
        let mut v = vec.as_mut_slice();
        let expected_c0 = first_chunk(&mut v);
        let h = Heap::new(&mut v);
        let c0 = h.first_chunk();

        assert_eq!(1, h.chunk_count());
        assert_eq!(expected_c0, c0);
        assert_eq!(32*1024 / Chunk::alignment(), c0.size());
        assert_eq!(true, c0.is_last());
    }

    // medium memory : 512Kib
    {
        let mut vec = setup(512*1014);
        let mut v = vec.as_mut_slice();
        let expected_c0 = first_chunk(&mut v);
        let h = Heap::new(&mut v);
        let expected_c1_size = (512*1014)/Chunk::alignment() - Chunk::max_size();
        let c0 = h.first_chunk();
        let c1 = c0.next().unwrap();

        assert_eq!(2, h.chunk_count());
        assert_eq!(expected_c0, c0);
        assert_eq!(Chunk::max_size(), c0.size());
        assert_eq!(false, c0.is_allocated());
        assert_eq!(false, c0.is_last());

        assert_eq!(expected_c1_size, c1.size());
        assert_eq!(false, c1.is_allocated());
        assert_eq!(true, c1.is_last());
    }

    // large memory : 8MiB
    {
        let mut vec = setup(8*1024*1024);
        let mut v = vec.as_mut_slice();
        let expected_c0 = first_chunk(&mut v);
        let h = Heap::new(&mut v);
        let c0 = h.first_chunk();

        let alignment_unit_count = 8*1024*1024 / Chunk::alignment();
        let mut expected_count = alignment_unit_count / Chunk::max_size();
        let spare = alignment_unit_count - expected_count*Chunk::max_size();
        if spare != 0 {
            expected_count += 1;
        }

        assert_eq!(expected_count, h.chunk_count());
        assert_eq!(expected_c0, c0);

        let mut prev_size = 0;
        let mut total = alignment_unit_count;
        let mut c = c0;
        for i in 0..h.chunk_count() {
            assert_eq!(prev_size, c.prev_size());
            assert_eq!(min(Chunk::max_size(), total), c.size());
            assert_eq!(false, c.is_allocated());
            assert_eq!(i == h.chunk_count() - 1, c.is_last());

            total -= c.size();
            prev_size = c.size();
            if i < h.chunk_count() - 1 {
                c = c.next().unwrap();
            }
        }
    }
}
