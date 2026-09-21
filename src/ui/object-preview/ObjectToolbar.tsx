import { useState, type ComponentProps } from "react";
import { demoObjects } from "./mock-objects";
import { ObjectToolbarControls } from "./ObjectToolbarControls";
import { ObjectImportDialog } from "./ObjectImportDialog";
import { ObjectGenerateDialog } from "./ObjectGenerateDialog";
import "./preview-object-toolbar.css";
import "./preview-object-dialogs.css";

const availableTags = [
  ...new Set(demoObjects.flatMap((object) => object.tags)),
];
type Props = Omit<
  ComponentProps<typeof ObjectToolbarControls>,
  "availableTags" | "importObjects" | "generateObjects"
> & { notify: (message: string) => void };

export function ObjectToolbar({ notify, ...controls }: Props) {
  const [importOpen, setImportOpen] = useState(false);
  const [generateOpen, setGenerateOpen] = useState(false);
  return (
    <>
      <ObjectToolbarControls
        {...controls}
        availableTags={availableTags}
        importObjects={() => setImportOpen(true)}
        generateObjects={() => setGenerateOpen(true)}
      />
      {importOpen && (
        <ObjectImportDialog
          close={() => setImportOpen(false)}
          notify={notify}
        />
      )}
      {generateOpen && (
        <ObjectGenerateDialog
          close={() => setGenerateOpen(false)}
          notify={notify}
        />
      )}
    </>
  );
}
