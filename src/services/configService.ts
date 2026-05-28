import type { AppConfig, AppStatePayload } from "../types";
import { callCommand } from "./tauri";

export const configService = {
  getAppState(): Promise<AppStatePayload> {
    return callCommand<AppStatePayload>("get_app_state");
  },

  completeOnboarding(): Promise<AppConfig> {
    return callCommand<AppConfig>("complete_onboarding");
  },

  saveConfig(config: AppConfig): Promise<AppConfig> {
    return callCommand<AppConfig>("save_app_config", {
      input: {
        companyName: config.companyName,
        taxId: config.taxId,
        thankYouMessage: config.thankYouMessage || null,
        validityDays: config.validityDays,
        theme: config.theme,
        printerName: config.printerName || null,
        printWidthChars: config.printWidthChars,
        setupCompleted: config.setupCompleted
      }
    });
  },

  openCreatorPortfolio(): Promise<void> {
    return callCommand<void>("open_creator_portfolio");
  }
};
