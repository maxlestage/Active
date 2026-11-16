# Changelog

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
