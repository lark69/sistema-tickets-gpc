import { FormEvent, useEffect, useState } from "react";
import { Button } from "../components/ui/Button";
import { Select } from "../components/ui/Select";
import { TextInput } from "../components/ui/TextInput";
import { adminService } from "../services/adminService";
import type { LocalUser } from "../types";
import { getErrorMessage } from "../utils/errors";

interface UsersPageProps {
  onMessage: (message: string, tone: "success" | "error" | "info") => void;
}

export function UsersPage({ onMessage }: UsersPageProps) {
  const [users, setUsers] = useState<LocalUser[]>([]);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState("operator");

  async function load() {
    setUsers(await adminService.listUsers());
  }

  async function create(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    try {
      await adminService.createUser(username, password, role);
      setUsername("");
      setPassword("");
      await load();
      onMessage("Usuário criado.", "success");
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    }
  }

  useEffect(() => {
    load().catch((err) => onMessage(getErrorMessage(err), "error"));
  }, []);

  return (
    <section className="page-stack">
      <div className="section-heading"><span>Admin</span><h1>Usuários e permissões</h1></div>
      <form className="settings-section" onSubmit={create}>
        <div className="form-grid">
          <TextInput label="Usuário" value={username} onChange={(e) => setUsername(e.target.value)} />
          <TextInput label="Senha" type="password" value={password} onChange={(e) => setPassword(e.target.value)} />
        </div>
        <Select label="Perfil" value={role} onChange={(e) => setRole(e.target.value)} options={[{ value: "operator", label: "Operador/Caixa" }, { value: "admin", label: "Admin" }]} />
        <Button type="submit">Criar usuário</Button>
      </form>
      <section className="logs-table">
        <div className="logs-row logs-row-head"><span>Usuário</span><span>Perfil</span><span>Status</span></div>
        {users.map((user) => (
          <div className="logs-row" key={user.id}><span>{user.username}</span><span>{user.role}</span><span>{user.active ? "Ativo" : "Inativo"}</span></div>
        ))}
      </section>
    </section>
  );
}
