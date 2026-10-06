# 校准来源边界

此生成输入只记录已转述为语义的几何比例。原始观测数据留在独立分析目录，不随生成配方传入。本轮为第一个结构校准候选，不视为视觉通过。


## Reference recheck for95 (2026-10-06)
The user rejected94's ear shape, jaw-to-ear contour, missing visible white eye point and excessive eye recess. Reference base atlas inspection plus per-component FBX UV samples show separate small eye-area components mapped to a near-white palette region; the primary iris atlas alone is not the whole visible eye. This supports an original independently parameterized catchlight surface rather than merely enlarging a subpixel atlas dot. The current authored95 uses its own quad patch, not those vertices/UVs.
Reference raw iris normalized depth ranges about-0.052 to-0.008H and includes multiple export pieces; missing original authoring modifiers cannot be reconstructed from this. Current94's posterior pocket uses a straight0.065H band after a0.020H setback.95 reduces those authored spans while preserving aperture and checking actual side views. Do not claim reference geometry necessarily has a shorter global bounding-box depth; the user is rejecting the visible recessed structure.
Ear silhouette reference maximum lateral extent is about0.460H, with upper fullness and a tapered low lobe;94's symmetric bulge and shared-root mechanism do not alone match it. Reference jaw posterior boundary also turns inward below the ear rather than staying at the slice's maximum width. These are sparse semantic observations for later contour parameters; no full mesh or texture is copied.
