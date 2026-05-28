interface KnownCommandError {
  message?: string;
  kind?: string;
}

export function getErrorMessage(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }

  if (error instanceof Error) {
    return error.message;
  }

  if (typeof error === "object" && error !== null) {
    const commandError = error as KnownCommandError;
    if (commandError.message) {
      return commandError.message;
    }
  }

  return "Ocorreu um erro inesperado. Tente novamente.";
}
