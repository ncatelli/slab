use slab::*;

const CHUNKS: usize = usize::BITS as usize;

fn main() {
    let mut slab = SlabAllocator::<u32, CHUNKS>::new();

    let boxed_values: Vec<_> = (0..(usize::BITS * 32) as u32)
        .into_iter()
        .map(|x| slab.boxed(x))
        .collect();

    println!("{:#?}", &boxed_values)
}
