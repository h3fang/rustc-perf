mod corpora;

use std::collections::{BinaryHeap, HashMap, LinkedList, VecDeque};
use std::hint::black_box;

use benchlib::benchmark::run_benchmark_group;

static LONG_HAYSTACK: &str = "\
Lorem ipsum dolor sit amet, consectetur adipiscing elit. Suspendisse quis lorem sit amet dolor \
ultricies condimentum. Praesent iaculis purus elit, ac malesuada quam malesuada in. Duis sed orci \
eros. Suspendisse sit amet magna mollis, mollis nunc luctus, imperdiet mi. Integer fringilla non \
sem ut lacinia. Fusce varius tortor a risus porttitor hendrerit. Morbi mauris dui, ultricies nec \
tempus vel, gravida nec quam.

In est dui, tincidunt sed tempus interdum, adipiscing laoreet ante. Etiam tempor, tellus quis \
sagittis interdum, nulla purus mattis sem, quis auctor erat odio ac tellus. In nec nunc sit amet \
diam volutpat molestie at sed ipsum. Vestibulum laoreet consequat vulputate. Integer accumsan \
lorem ac dignissim placerat. Suspendisse convallis faucibus lorem. Aliquam erat volutpat. In vel \
eleifend felis. Sed suscipit nulla lorem, sed mollis est sollicitudin et. Nam fermentum egestas \
interdum. Curabitur ut nisi justo.

Sed sollicitudin ipsum tellus, ut condimentum leo eleifend nec. Cras ut velit ante. Phasellus nec \
mollis odio. Mauris molestie erat in arcu mattis, at aliquet dolor vehicula. Quisque malesuada \
lectus sit amet nisi pretium, a condimentum ipsum porta. Morbi at dapibus diam. Praesent egestas \
est sed risus elementum, eu rutrum metus ultrices. Etiam fermentum consectetur magna, id rutrum \
felis accumsan a. Aliquam ut pellentesque libero. Sed mi nulla, lobortis eu tortor id, suscipit \
ultricies neque. Morbi iaculis sit amet risus at iaculis. Praesent eget ligula quis turpis \
feugiat suscipit vel non arcu. Interdum et malesuada fames ac ante ipsum primis in faucibus. \
Aliquam sit amet placerat lorem.

Cras a lacus vel ante posuere elementum. Nunc est leo, bibendum ut facilisis vel, bibendum at \
mauris. Nullam adipiscing diam vel odio ornare, luctus adipiscing mi luctus. Nulla facilisi. \
Mauris adipiscing bibendum neque, quis adipiscing lectus tempus et. Sed feugiat erat et nisl \
lobortis pharetra. Donec vitae erat enim. Nullam sit amet felis et quam lacinia tincidunt. Aliquam \
suscipit dapibus urna. Sed volutpat urna in magna pulvinar volutpat. Phasellus nec tellus ac diam \
cursus accumsan.

Nam lectus enim, dapibus non nisi tempor, consectetur convallis massa. Maecenas eleifend dictum \
feugiat. Etiam quis mauris vel risus luctus mattis a a nunc. Nullam orci quam, imperdiet id \
vehicula in, porttitor ut nibh. Duis sagittis adipiscing nisl vitae congue. Donec mollis risus eu \
leo suscipit, varius porttitor nulla porta. Pellentesque ut sem nec nisi euismod vehicula. Nulla \
malesuada sollicitudin quam eu fermentum.";

/// Returns a `rand::Rng` seeded with a consistent seed.
///
/// This is done to avoid introducing nondeterminism in benchmark results.
fn bench_rng() -> rand_xorshift::XorShiftRng {
    const SEED: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    rand::SeedableRng::from_seed(SEED)
}

macro_rules! repeat {
    ($s: expr) => {
        concat!($s, $s, $s, $s, $s, $s, $s, $s, $s, $s)
    };
}

const MEDIUM: &str = "Alice's Adventures in Wonderland";
const LONG: &str = repeat!(
    r#"
    La Guida di Bragia, a Ballad Opera for the Marionette Theatre (around 1850)
    Alice's Adventures in Wonderland (1865)
    Phantasmagoria and Other Poems (1869)
    Through the Looking-Glass, and What Alice Found There
        (includes "Jabberwocky" and "The Walrus and the Carpenter") (1871)
    The Hunting of the Snark (1876)
    Rhyme? And Reason? (1883) – shares some contents with the 1869 collection,
        including the long poem "Phantasmagoria"
    A Tangled Tale (1885)
    Sylvie and Bruno (1889)
    Sylvie and Bruno Concluded (1893)
    Pillow Problems (1893)
    What the Tortoise Said to Achilles (1895)
    Three Sunsets and Other Poems (1898)
    The Manlet (1903)[106]
"#
);

fn case00_libcore(bytes: &[u8]) -> bool {
    bytes.is_ascii()
}

fn case04_while_loop(bytes: &[u8]) -> bool {
    // Process chunks of 32 bytes at a time in the fast path to enable
    // auto-vectorization and use of `pmovmskb`. Two 128-bit vector registers
    // can be OR'd together and then the resulting vector can be tested for
    // non-ASCII bytes.
    const CHUNK_SIZE: usize = 32;

    let mut i = 0;

    while i + CHUNK_SIZE <= bytes.len() {
        let chunk_end = i + CHUNK_SIZE;

        // Get LLVM to produce a `pmovmskb` instruction on x86-64 which
        // creates a mask from the most significant bit of each byte.
        // ASCII bytes are less than 128 (0x80), so their most significant
        // bit is unset.
        let mut count = 0;
        while i < chunk_end {
            count += bytes[i].is_ascii() as u8;
            i += 1;
        }

        // All bytes should be <= 127 so count is equal to chunk size.
        if count != CHUNK_SIZE as u8 {
            return false;
        }
    }

    // Process the remaining `bytes.len() % N` bytes.
    let mut is_ascii = true;
    while i < bytes.len() {
        is_ascii &= bytes[i].is_ascii();
        i += 1;
    }

    is_ascii
}

fn main() {
    run_benchmark_group(|group| {
        // Top10 decreased performance
        group.register_benchmark("str::bench_contains_bad_naive", || {
            let haystack =
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
            let needle = "aaaaaaaab";
            move || {
                assert!(!black_box(haystack).contains(black_box(needle)));
            }
        });
        group.register_benchmark("str::bench_contains_2b_repeated_long", || {
            let haystack = LONG_HAYSTACK;
            let needle = "::";
            move || {
                assert!(!black_box(haystack).contains(black_box(needle)));
            }
        });
        group.register_benchmark("str::bench_contains_short_long", || {
            let haystack = LONG_HAYSTACK;
            let needle = "english";

            move || {
                assert!(!black_box(haystack).contains(black_box(needle)));
            }
        });
        group.register_benchmark("hash::map::new_drop", || {
            move || {
                let m: HashMap<i32, i32> = HashMap::new();
                assert_eq!(black_box(m).len(), 0);
            }
        });
        group.register_benchmark("binary_heap::bench_from_vec", || {
            use rand::seq::SliceRandom;

            let mut rng = crate::bench_rng();
            let mut vec: Vec<u32> = (0..100_000).collect();
            vec.shuffle(&mut rng);

            || BinaryHeap::from(vec)
        });
        group.register_benchmark("str::bench_contains_16b_in_long", || {
            let haystack = LONG_HAYSTACK;
            let needle = "english language";

            move || {
                assert!(!black_box(haystack).contains(black_box(needle)));
            }
        });
        group.register_benchmark("str::bench_contains_32b_in_long", || {
            let haystack = LONG_HAYSTACK;
            let needle = "the english language sample text";

            move || {
                assert!(!black_box(haystack).contains(black_box(needle)));
            }
        });
        group.register_benchmark(
            "ascii::is_ascii::unaligned_both_long::case04_while_loop",
            || {
                let mut vec = LONG.as_bytes().to_vec();
                move || {
                    let arg: &[u8] = &black_box(&mut vec)[1..(LONG.len() - 1)];
                    black_box(case04_while_loop(arg))
                }
            },
        );
        group.register_benchmark(
            "ascii::is_ascii::unaligned_tail_long::case04_while_loop",
            || {
                let mut vec = LONG.as_bytes().to_vec();
                move || {
                    let arg: &[u8] = &black_box(&mut vec)[..(LONG.len() - 1)];
                    black_box(case04_while_loop(arg))
                }
            },
        );
        group.register_benchmark("str::char_count::ru_huge::case04_while_loop", || {
            fn manual_char_len(s: &str) -> usize {
                let s = s.as_bytes();
                let mut c = 0;
                let mut i = 0;
                let l = s.len();
                while i < l {
                    let b = s[i];
                    if b < 0x80 {
                        i += 1;
                    } else if b < 0xe0 {
                        i += 2;
                    } else if b < 0xf0 {
                        i += 3;
                    } else {
                        i += 4;
                    }
                    c += 1;
                }
                c
            }

            fn case03_manual_char_len(s: &str) -> usize {
                manual_char_len(s)
            }

            move || {
                let input = corpora::ru::HUGE;
                let mut input_s = input.to_string();
                move || {
                    let arg: &str = &black_box(&mut input_s);
                    black_box(case03_manual_char_len(arg))
                }
            }
        });

        // Top10 increased performance
        group.register_benchmark("slice::reverse_u8", || {
            // odd length and offset by 1 to be as unaligned as possible
            let n = 0xFFFFF;
            let mut v: Vec<_> = (0..1 + (n / size_of::<u8>() as u64))
                .map(|x| x as u8)
                .collect();
            move || black_box(&mut v[1..]).reverse()
        });
        group.register_benchmark("num::int_sqrt::u16_sqrt_predictable", || {
            || {
                for n in 0..(u16::BITS / 8) {
                    for i in 1..=(100 as u16) {
                        let x = black_box(i << (n * 8));
                        black_box(x.isqrt());
                    }
                }
            }
        });
        group.register_benchmark("vec::bench_extend_from_slice_0000_0000", || {
            let (dst_len, src_len) = (0, 0);
            let dst: Vec<_> = FromIterator::from_iter(0..0);
            let src: Vec<_> = FromIterator::from_iter(dst_len..dst_len + src_len);

            move || {
                let mut dst = dst.clone();
                dst.extend_from_slice(&src);
                dst
            }
        });
        group.register_benchmark(
            "ascii::is_ascii::unaligned_tail_medium::case00_libcore",
            || {
                let mut vec = MEDIUM.as_bytes().to_vec();
                move || {
                    let arg: &[u8] = &black_box(&mut vec)[..(MEDIUM.len() - 1)];
                    black_box(case00_libcore(arg))
                }
            },
        );
        group.register_benchmark("num::int_log::u8_log10_predictable", || {
            || {
                for n in 0..(u8::BITS / 8) {
                    for i in 1..=(100 as u8) {
                        let x = black_box(i << (n * 8));
                        black_box(x.ilog10());
                    }
                }
            }
        });
        group.register_benchmark("vec::bench_extend_chained_trustedlen", || {
            let mut ring: VecDeque<u16> = VecDeque::with_capacity(1000);

            move || {
                ring.clear();
                ring.extend(black_box((0..256).chain(768..1024)));
            }
        });
        group.register_benchmark("vec::bench_flat_map_collect", || {
            let v = vec![777u32; 500000];
            move || {
                v.iter()
                    .flat_map(|color| color.rotate_left(8).to_be_bytes())
                    .collect::<Vec<_>>()
            }
        });
        group.register_benchmark(
            "ascii::is_ascii::unaligned_both_medium::case00_libcore",
            || {
                let mut vec = MEDIUM.as_bytes().to_vec();
                move || {
                    let arg: &[u8] = &black_box(&mut vec)[1..(MEDIUM.len() - 1)];
                    black_box(case00_libcore(arg))
                }
            },
        );
        group.register_benchmark("linked_list::bench_push_front", || {
            let mut m: LinkedList<_> = LinkedList::new();
            move || {
                m.push_front(0);
            }
        });
        group.register_benchmark("str::contains_bang_char::short_mixed", || {
            let mut s = "ศไทย中华Việt Nam; Mary had a little lamb, Little lam!";
            black_box(&mut s);
            move || {
                for _ in 0..1 {
                    black_box(s.contains('!'));
                }
            }
        });
    });
}
