import { Printer, X } from "lucide-react";
import { FormEvent, useState } from "react";
import type { Product } from "../../types";
import { formatCurrency } from "../../utils/currency";
import { Button } from "../ui/Button";
import { Modal } from "../ui/Modal";
import { TextInput } from "../ui/TextInput";

interface PrintModalProps {
  product: Product;
  printing: boolean;
  onClose: () => void;
  onPrint: (quantity: number) => Promise<void>;
}

export function PrintModal({ product, printing, onClose, onPrint }: PrintModalProps) {
  const [quantity, setQuantity] = useState("1");
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const parsedQuantity = Number.parseInt(quantity, 10);

    if (!Number.isInteger(parsedQuantity) || parsedQuantity < 1 || parsedQuantity > 999) {
      setError("Informe uma quantidade entre 1 e 999.");
      return;
    }

    setError(null);
    await onPrint(parsedQuantity);
  }

  return (
    <Modal
      title="Imprimir ticket"
      onClose={onClose}
      footer={
        <>
          <Button
            type="submit"
            form="print-ticket-form"
            icon={<Printer size={18} />}
            loading={printing}
          >
            Imprimir
          </Button>
          <Button type="button" variant="secondary" icon={<X size={18} />} onClick={onClose}>
            Cancelar
          </Button>
        </>
      }
    >
      <form id="print-ticket-form" className="print-modal-content" onSubmit={handleSubmit}>
        <div className="ticket-preview">
          <span>Produto</span>
          <strong>{product.name}</strong>
          <em>{formatCurrency(product.priceCents)}</em>
        </div>
        <TextInput
          label="Quantidade de tickets"
          type="number"
          min={1}
          max={999}
          value={quantity}
          onChange={(event) => setQuantity(event.target.value)}
        />
        {error ? <div className="inline-alert">{error}</div> : null}
      </form>
    </Modal>
  );
}
