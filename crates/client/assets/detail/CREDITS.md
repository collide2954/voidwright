# Surface credits

| Channel | Work | Author | Source | License |
| --- | --- | --- | --- | --- |
| Red: woven cloth | Poly Wool Herringbone | colormass, Rico Cilliers | [Poly Haven](https://polyhaven.com/a/poly_wool_herringbone) | CC0 1.0 |
| Green: knitted wool | Knitted Fleece | colormass, Rico Cilliers | [Poly Haven](https://polyhaven.com/a/knitted_fleece) | CC0 1.0 |
| Blue: grained leather | Fabric Leather 01 | Rob Tuytel | [Poly Haven](https://polyhaven.com/a/fabric_leather_01) | CC0 1.0 |
| Alpha: brushed metal | Metal Plate 02 | Rob Tuytel | [Poly Haven](https://polyhaven.com/a/metal_plate_02) | CC0 1.0 |

Each file holds 512 × 512 pixels of 8-bit RGBA without a header. One channel holds one surface.

- `tone.rgba` holds the brightness of the color map.
- `slope_u.rgba` and `slope_v.rgba` hold the red and the green channels of the OpenGL normal map.
- `rough.rgba` holds the roughness map.

The conversion scaled the 1K JPEG maps to 512 × 512 pixels with a Lanczos filter.
