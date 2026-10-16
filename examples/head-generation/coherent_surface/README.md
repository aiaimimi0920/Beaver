# Packaging-time surface-fit helpers

Constrained cubic-profile fitting, bounded profile evaluation and analytic diagnostics used by `../facial_surface`. These CPU helpers do not execute an engine or certify final mesh appearance. NumPy and SciPy are packaging dependencies; the native request carries the fitted coefficients and uses pure Python. The old coherent672 standalone packager is not part of this change.
