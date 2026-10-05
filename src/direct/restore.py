"""Restore approved asset backup only; never restore task databases or credentials."""
import sys,zipfile,json,hashlib
from pathlib import Path
archive,root=map(Path,sys.argv[1:3]);report=[]
with zipfile.ZipFile(archive) as z:
 for item in z.infolist():
  p=Path(item.filename)
  if item.is_dir():continue
  if p.is_absolute() or '..' in p.parts:raise ValueError('Unsafe archive path')
  if p.parts[0]!='project':continue
  p=Path(*p.parts[1:])
  if any(x.startswith('.') for x in p.parts):continue
  if p.parts[0]=='addons' or p.name in ('beaver.runtime.json','beaver.validation.json'):continue
  if item.external_attr>>16&0o170000==0o120000:raise ValueError('Symlink archive entry')
  dest=root/p;raw=z.read(item);dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(raw)
  report.append({'path':str(p),'sha256':hashlib.sha256(raw).hexdigest()})
(root/'recovery-import.json').write_text(json.dumps({'source_archive_sha256':hashlib.sha256(archive.read_bytes()).hexdigest(),'files':report,'note':'Approved older baseline; not the interrupted candidate02'},indent=2))
print(json.dumps({'restored_files':len(report),'archive_sha256':hashlib.sha256(archive.read_bytes()).hexdigest()}))
