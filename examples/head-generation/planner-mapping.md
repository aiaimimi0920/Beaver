Only explicit brow/iris/lower-cover and nose pigment controls change from74. Treat pigment height separately from geometric nose height. Verify front render versus white view; do not claim geometry changed when only the atlas changed.

## Reconstructed81 mapping, written before implementation
- Useful76 parameters: upper lip peak height0.176H, influence width0.020H, relief0.024H; lower relief0.003H; mouth height0.155H.
- Useful79 neutral aperture: half-gap0.00065H, total0.3068mm atH0.236m; upper rim depth delta+0.008H, lower-0.004H, smoothly tapered at corners and across existing rings.
- Planned80 correction: one curved rest seam y(x)=mouth_height+corner_lift*clamp(x/half_width,-1,1)^2. Aperture, jaw classification/falloff and cavity normalization share this function. This was not executed before reset and requires fresh tests.
- Keep eye/mouth boundary crease1.0;77 changed no exported positions and is not treated as a useful geometry control.
- Do not restore78's rejected wider/deeper neutral dark band.
