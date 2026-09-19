import { X } from "lucide-react";
import { useEffect, useRef } from "react";

export function ImagePreviewModal({
  preview,
  onClose,
}: {
  preview: { dataUrl: string; alt: string; caption: string };
  onClose: () => void;
}) {
  const dialogRef = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = dialogRef.current;
    const opener = document.activeElement;
    dialog?.showModal();
    return () => {
      dialog?.close();
      if (opener instanceof HTMLElement && opener.isConnected) opener.focus();
    };
  }, []);

  return (
    <dialog
      ref={dialogRef}
      className="preview-modal"
      aria-label={preview.caption}
      onCancel={(event) => { event.preventDefault(); onClose(); }}
      onClick={(event) => { if (event.target === event.currentTarget) onClose(); }}
    >
      <figure className="preview-frame">
        <button className="preview-close" type="button" onClick={onClose} aria-label="Close preview" autoFocus>
          <X />
        </button>
        <img src={preview.dataUrl} alt={preview.alt} />
        <figcaption>{preview.caption}</figcaption>
      </figure>
    </dialog>
  );
}
