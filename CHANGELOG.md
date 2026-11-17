# Changelog

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
