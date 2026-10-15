# Historical code-size exception resolved

Candidate 388 extracted reusable thin-ribbon construction but left the integrated
lash module oversized. The automatic-delivery repair now extracts shared attachment,
hinge and crown fitting into `accent_attachment.py`. All 33 original function ASTs
are frozen by `accent-attachment-refactor-contract.json`; 12 moved unchanged.

Every Python source declared by the current bundle is at most 500 nonblank,
noncomment lines even when docstrings are counted. No generated-path exception or
reserved configuration write is required. The old repository exception is removed.

Run `python -m unittest discover -s examples/head-generation/checkpoints/current
-p 'test_*.py'` from the repository root, then `npm run check:effective-lines` in an
environment with Cargo. Native model generation and output acceptance remain a
separate verification step; this source-only repair does not claim that result.
