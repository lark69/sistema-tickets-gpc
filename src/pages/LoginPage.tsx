import { FormEvent, useState } from "react";
import { Button } from "../components/ui/Button";
import { TextInput } from "../components/ui/TextInput";
import { adminService } from "../services/adminService";
import type { LocalUser } from "../types";
import { getErrorMessage } from "../utils/errors";

interface LoginPageProps {
  onLogin: (user: LocalUser) => void;
  onMessage: (message: string, tone: "success" | "error" | "info") => void;
}

export function LoginPage({ onLogin, onMessage }: LoginPageProps) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setLoading(true);
    try {
      const result = await adminService.login(username, password);
      onLogin(result.user);
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
          <h1>Entrar no sistema</h1>
          <p>Use um usuário administrador ou operador para acessar o caixa.</p>
        </div>
        <TextInput label="Usuário" value={username} onChange={(event) => setUsername(event.target.value)} />
        <TextInput
          label="Senha"
          type="password"
          value={password}
          onChange={(event) => setPassword(event.target.value)}
        />
        <div className="form-actions">
          <Button type="submit" loading={loading}>
            Entrar
          </Button>
        </div>
      </form>
    </main>
  );
}
