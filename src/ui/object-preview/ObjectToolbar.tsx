import { useState, type ComponentProps } from "react";
import { demoObjects } from "./mock-objects";
import { ObjectToolbarControls } from "./ObjectToolbarControls";
import { ObjectImportDialog } from "./ObjectImportDialog";
import "./preview-object-toolbar.css";
import "./preview-object-dialogs.css";

const availableTags = [
  ...new Set(demoObjects.flatMap((object) => object.tags)),
];
type Props = Omit<
  ComponentProps<typeof ObjectToolbarControls>,
  "availableTags" | "importObjects" | "generateObjects"
> & { notify: (message: string) => void; projectId?: string };

export function ObjectToolbar({ notify, projectId, ...controls }: Props) {
  const [importOpen, setImportOpen] = useState(false);
  return (
    <>
      <ObjectToolbarControls
        {...controls}
        availableTags={availableTags}
        importObjects={projectId ? () => setImportOpen(true) : undefined}
      />
      {importOpen && (
        <ObjectImportDialog
          close={() => setImportOpen(false)}
          notify={notify}
          projectId={projectId}
        />
      )}
    </>
  );
}
