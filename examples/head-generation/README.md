# Original semantic head-generation recipe

Current review candidate:185. See review-results.md for actual checks and limitations.

Workflow: owner natural language → authoring brief → semantic planner mapping → face_style.json → packaged Beaver generation → native white/wire/render review. Do not run Blender from this packaging script. Generate bundle.py --output request.json and pass it through the existing Beaver blender.start tool.

Run local pure regressions with python3 -m unittest discover -q in this directory. The native project and private reference assets are not included in this source example.

The accepted baseline eye/head parameter freeze is checked by frozen177-contract.json. Eye-accent geometry is original, parameterized and kept separate from frozen regions. No reference mesh/UV/pixels enter the generator.
