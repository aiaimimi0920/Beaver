"""Bind one complete-guide fit to exact source-owned scaled Jacobian reuse."""
from forehead_cache_manifest import forehead_cache_source_manifest
from forehead_cache_graph import forehead_cache_bind_epoch
from forehead_exact_jacobian_cache import forehead_exact_fit_cache
from forehead_cache_schema import FOREHEAD_CACHE_NAMES


def forehead_install_jacobian_cache(stage, derivative, fit_state, fit_globals):
    if stage is None:
        return derivative, None
    state = {name: fit_state[name] for name in FOREHEAD_CACHE_NAMES}
    manifest = forehead_cache_source_manifest(fit_globals, derivative)
    epoch = forehead_cache_bind_epoch(state, derivative, manifest)

    def charge_request(kind):
        # Preserve the old public-request charge on hits and rejected requests.
        stage.charge('scaled_constraint_derivative_including_cache')

    cached, metadata = forehead_exact_fit_cache(
        derivative, epoch, charge_request,
        len(state['scales']), len(state['ids']) * 2)
    return cached, metadata
