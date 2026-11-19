# Changelog

## 0.3.4

Faster server-side rendering of classes and faster reactive updates, same output and same
behavior. Measured natively (release profile), median of 9 interleaved runs, on a page of
200 product cards (2,000 elements with classes and attributes, 72 KB) and of 20 cards:

| Case | 0.3.3 | 0.3.4 | Gain |
| --- | ---: | ---: | ---: |
| 200 cards: build + render + drop | 234.3 µs | 216.5 µs | −8 % |
| 200 cards: render | 76.5 µs | 61.4 µs | −20 % |
| 200 cards: drop | 39.1 µs | 33.8 µs | −14 % |
| 200 cards: build + drop, in a loop | 215.1 µs | 194.2 µs | −10 % |
| 20 cards: build + render + drop | 23.8 µs | 21.0 µs | −11 % |

And a `State::set` with its effects (the `client` feature compiled natively, median of 7
runs of 15 batches):

| Case | 0.3.3 | 0.3.4 | Gain |
| --- | ---: | ---: | ---: |
| One state read by one effect | 68.2 ns | 40.1 ns | −41 % |
| One state read by ten effects | 584 ns | 376 ns | −36 % |
| A memo read by three effects | 239.8 ns | 147.9 ns | −38 % |
| One effect reading four states | 105.9 ns | 76.2 ns | −28 % |

- An element keeps its static classes in one string, joined by spaces as they are added:
  an element with one class no longer allocates a list, and the server writes the classes
  with one escaping call. The HTML written is byte for byte the same (static export of the
  site compared file by file with 0.3.3).
- A changed state marks its readers by reading its list of subscribers in place instead of
  copying it; an effect that runs reuses the memory of its list of dependencies instead of
  allocating a new one, and its dependencies are only copied when memos have to be checked.
  Effects run in the same order, as many times.
- The site's WebAssembly bundle is 1,051 bytes smaller.

## 0.3.3

Faster HTML escaping, above all when special characters are frequent (0.3.2 had made that
case slower than 0.3.1), same output. Measured natively (release profile), median of 7
interleaved runs, on a page of 400 paragraphs rendered by `Document::render` (text with no
special character, or with one character in eight or one in two among `& " < >`):

| Case | 0.3.1 | 0.3.2 | 0.3.3 | Gain on 0.3.2 |
| --- | ---: | ---: | ---: | ---: |
| No special character (42 KB) | 124.8 µs | 107.1 µs | 105.8 µs | −1 % |
| One in eight (57 KB) | 168.6 µs | 185.3 µs | 155.8 µs | −16 % |
| One in two (105 KB) | 266.9 µs | 305.0 µs | 269.1 µs | −12 % |

And the escaping alone (`escape_into` on 34 KB as one string, median of 3 × 15 interleaved
runs; the documentation text is the site's English Markdown):

| Case | 0.3.1 | 0.3.2 | 0.3.3 | Gain on 0.3.2 |
| --- | ---: | ---: | ---: | ---: |
| No special character | 10.9 µs | 11.9 µs | 3.4 µs | −71 % |
| Documentation text (1 in 58) | 19.3 µs | 15.6 µs | 5.9 µs | −62 % |
| One in eight | 47.4 µs | 54.7 µs | 30.2 µs | −45 % |
| One in two | 133.1 µs | 154.4 µs | 116.1 µs | −25 % |

- HTML escaping turns each block of 16 bytes into a bit mask of its special characters and
  takes them from the mask one after the other, instead of testing the block again and then
  looking byte by byte for each one. A block without special characters still costs one
  test. The HTML written is byte for byte the same.

## 0.3.2

Faster server-side rendering, same output. Measured natively (release profile) with the
repository's SSR benchmark (`bench`, `ssr-active`: the product page, head and body), median of
5 interleaved runs:

| Case | 0.3.1 | 0.3.2 | Gain |
| --- | ---: | ---: | ---: |
| SSR product page (1.9 KB) | 5.53 µs | 5.17 µs | −6 % |

- HTML escaping (text, attributes and `<head>` tags) tests 16 bytes at a time for the five
  special characters, instead of looking each byte up in a table, and only looks byte by
  byte inside a block that has one. The HTML written is byte for byte the same.

## 0.3.1

Faster ECS, same behavior. Measured natively (release profile) on `App::update` with
`MinimalPlugins` + `TransformPlugin`, or `DefaultPlugins`, median of 3 interleaved runs:

| Case | 0.3.0 | 0.3.1 | Gain |
| --- | ---: | ---: | ---: |
| 5 000 flat entities (spin + transforms) | 439 µs | 154 µs | −65 % |
| 5 100 entities in a hierarchy (100 × 10 × 4) | 783 µs | 196 µs | −75 % |
| 2 000 meshes with physics (`DefaultPlugins`) | 3.11 ms | 2.40 ms | −23 % |

- Component, resource and observer maps are keyed by `TypeId` with a hasher that uses its
  bits as they are, instead of hashing them again with SipHash.
- A `Query` looks up its columns once, when it is created, instead of on every `get`,
  `contains`, `single` and `iter`.
- Transform and visibility propagation read each entity's `Children` in place instead of
  copying them into a new `Vec` at every node.
