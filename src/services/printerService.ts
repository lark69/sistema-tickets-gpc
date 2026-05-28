import type { PrintResult, PrintTicketsInput, PrinterInfo, VerifyTicketResult } from "../types";
import { callCommand } from "./tauri";

export const printerService = {
  listPrinters(): Promise<PrinterInfo[]> {
    return callCommand<PrinterInfo[]>("list_printers");
  },

  printTickets(input: PrintTicketsInput): Promise<PrintResult> {
    return callCommand<PrintResult>("print_tickets", { input });
  },

  verifyTicket(ticketId: string): Promise<VerifyTicketResult> {
    return callCommand<VerifyTicketResult>("verify_ticket", {
      input: { ticketId }
    });
  }
};
