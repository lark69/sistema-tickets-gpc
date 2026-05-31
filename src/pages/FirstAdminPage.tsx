import { FormEvent, useState } from "react";
import { Button } from "../components/ui/Button";
import { TextInput } from "../components/ui/TextInput";
import { adminService } from "../services/adminService";
import type { LocalUser } from "../types";
import { getErrorMessage } from "../utils/errors";
import { ALL_USER_PERMISSIONS } from "../utils/permissions";

interface FirstAdminPageProps {
  onCreated: (user: LocalUser) => void;
  onMessage: (message: string, tone: "success" | "error" | "info") => void;
}

export function FirstAdminPage({ onCreated, onMessage }: FirstAdminPageProps) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!username.trim()) {
      setError("Informe o nome de usuario.");
      return;
    }

    if (password.length < 4) {
      setError("A senha precisa ter pelo menos 4 caracteres.");
      return;
    }

    if (password !== confirmPassword) {
      setError("As senhas nao conferem.");
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const user = await adminService.createUser(username, password, "admin", ALL_USER_PERMISSIONS, null);
      onCreated(user);
      onMessage("Administrador criado com sucesso.", "success");
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setLoading(false);
    }
  }

  return (
    <main className="setup-screen">
      <form className="setup-panel" onSubmit={handleSubmit}>
        <div className="section-heading">
          <span>Portex PDV</span>
          <h1>Criar primeiro acesso</h1>
          <p>Defina o usuario administrador que tera acesso total ao sistema.</p>
        </div>

        <TextInput
          label="Usuario administrador"
          value={username}
          onChange={(event) => setUsername(event.target.value)}
          placeholder="Ex: admin"
          autoFocus
        />
        <TextInput
          label="Senha"
          type="password"
          value={password}
          onChange={(event) => setPassword(event.target.value)}
        />
        <TextInput
          label="Confirmar senha"
          type="password"
          value={confirmPassword}
          onChange={(event) => setConfirmPassword(event.target.value)}
        />

        {error ? <div className="inline-alert">{error}</div> : null}

        <div className="form-actions">
          <Button type="submit" loading={loading}>
            Criar administrador
          </Button>
        </div>
      </form>
    </main>
  );
}
