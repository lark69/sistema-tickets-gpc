import { FormEvent, useEffect, useState } from "react";
import { Button } from "../components/ui/Button";
import { Select } from "../components/ui/Select";
import { TextInput } from "../components/ui/TextInput";
import { adminService } from "../services/adminService";
import type { LocalUser, UserPermission } from "../types";
import { getErrorMessage } from "../utils/errors";
import {
  ALL_USER_PERMISSIONS,
  DEFAULT_OPERATOR_PERMISSIONS,
  USER_PERMISSION_OPTIONS,
  hasPermission,
  permissionsForRole
} from "../utils/permissions";

interface UsersPageProps {
  currentUser: LocalUser;
  onMessage: (message: string, tone: "success" | "error" | "info") => void;
}

const roleOptions = [
  { value: "operator", label: "Operador/Caixa" },
  { value: "admin", label: "Admin" }
];

function nextPermissionsForRole(role: string, current: UserPermission[]) {
  return role === "admin" ? ALL_USER_PERMISSIONS : current.length > 0 ? current : DEFAULT_OPERATOR_PERMISSIONS;
}

interface PermissionChecklistProps {
  role: string;
  value: UserPermission[];
  onChange: (permissions: UserPermission[]) => void;
}

function PermissionChecklist({ role, value, onChange }: PermissionChecklistProps) {
  const effectiveValue = permissionsForRole(role, value);
  const disabled = role === "admin";

  function toggle(permission: UserPermission) {
    if (disabled) return;

    onChange(
      value.includes(permission)
        ? value.filter((item) => item !== permission)
        : [...value, permission]
    );
  }

  return (
    <div className="permissions-grid">
      {USER_PERMISSION_OPTIONS.map((option) => (
        <label key={option.value} className="permission-option">
          <input
            type="checkbox"
            checked={effectiveValue.includes(option.value)}
            disabled={disabled}
            onChange={() => toggle(option.value)}
          />
          <span>
            <strong>{option.label}</strong>
            <small>{option.description}</small>
          </span>
        </label>
      ))}
      {disabled ? <p className="muted-text">Administradores possuem acesso total automaticamente.</p> : null}
    </div>
  );
}

export function UsersPage({ currentUser, onMessage }: UsersPageProps) {
  const [users, setUsers] = useState<LocalUser[]>([]);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState("operator");
  const [permissions, setPermissions] = useState<UserPermission[]>(DEFAULT_OPERATOR_PERMISSIONS);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editingUsername, setEditingUsername] = useState("");
  const [editingPassword, setEditingPassword] = useState("");
  const [editingRole, setEditingRole] = useState("operator");
  const [editingPermissions, setEditingPermissions] = useState<UserPermission[]>(DEFAULT_OPERATOR_PERMISSIONS);
  const [saving, setSaving] = useState(false);

  async function load() {
    setUsers(await adminService.listUsers());
  }

  async function create(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSaving(true);

    try {
      await adminService.createUser(
        username,
        password,
        role,
        permissionsForRole(role, permissions),
        currentUser
      );
      setUsername("");
      setPassword("");
      setRole("operator");
      setPermissions(DEFAULT_OPERATOR_PERMISSIONS);
      await load();
      onMessage("Usuario criado.", "success");
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setSaving(false);
    }
  }

  async function update(user: LocalUser) {
    setSaving(true);

    try {
      await adminService.updateUser({
        id: user.id,
        username: editingUsername,
        password: editingPassword || null,
        role: editingRole,
        permissions: permissionsForRole(editingRole, editingPermissions),
        requester: currentUser
      });
      setEditingId(null);
      setEditingPassword("");
      await load();
      onMessage("Usuario atualizado.", "success");
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setSaving(false);
    }
  }

  async function remove(user: LocalUser) {
    if (user.id === currentUser.id) {
      onMessage("Voce nao pode excluir o usuario em uso.", "error");
      return;
    }

    const confirmed = window.confirm(`Excluir o usuario "${user.username}"?`);
    if (!confirmed) {
      return;
    }

    setSaving(true);

    try {
      await adminService.deleteUser(user.id, currentUser);
      await load();
      onMessage("Usuario excluido.", "success");
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setSaving(false);
    }
  }

  useEffect(() => {
    load().catch((err) => onMessage(getErrorMessage(err), "error"));
  }, []);

  if (!hasPermission(currentUser, "manageUsers")) {
    return <section className="empty-state"><h2>Acesso restrito ao gerenciamento de usuarios.</h2></section>;
  }

  return (
    <section className="page-stack">
      <div className="section-heading">
        <span>Admin</span>
        <h1>Usuarios e permissoes</h1>
        <p>Controle exatamente o que cada usuario pode acessar no Portex PDV.</p>
      </div>

      <form className="settings-section" onSubmit={create}>
        <div className="settings-section-title">
          <h2>Novo usuario</h2>
          <p>Escolha perfil, senha inicial e permissoes operacionais.</p>
        </div>
        <div className="form-grid">
          <TextInput label="Usuario" value={username} onChange={(e) => setUsername(e.target.value)} />
          <TextInput label="Senha" type="password" value={password} onChange={(e) => setPassword(e.target.value)} />
        </div>
        <Select
          label="Perfil"
          value={role}
          onChange={(e) => {
            const nextRole = e.target.value;
            setRole(nextRole);
            setPermissions((current) => nextPermissionsForRole(nextRole, current));
          }}
          options={roleOptions}
        />
        <PermissionChecklist role={role} value={permissions} onChange={setPermissions} />
        <Button type="submit" loading={saving}>Criar usuario</Button>
      </form>

      <section className="logs-table users-table">
        <div className="logs-row logs-row-head users-row"><span>Usuario</span><span>Perfil</span><span>Permissoes</span><span>Acoes</span></div>
        {users.map((user) => (
          <div className="logs-row user-row users-row" key={user.id}>
            {editingId === user.id ? (
              <span className="user-edit-fields">
                <TextInput label="Usuario" value={editingUsername} onChange={(event) => setEditingUsername(event.target.value)} />
                <TextInput label="Nova senha" type="password" value={editingPassword} onChange={(event) => setEditingPassword(event.target.value)} placeholder="Deixe em branco para manter" />
              </span>
            ) : (
              <span>{user.username}{user.id === currentUser.id ? " (voce)" : ""}</span>
            )}

            {editingId === user.id ? (
              <Select
                label="Perfil"
                value={editingRole}
                onChange={(event) => {
                  const nextRole = event.target.value;
                  setEditingRole(nextRole);
                  setEditingPermissions((current) => nextPermissionsForRole(nextRole, current));
                }}
                options={roleOptions}
              />
            ) : (
              <span>{user.role === "admin" ? "Admin" : "Operador/Caixa"}</span>
            )}

            {editingId === user.id ? (
              <PermissionChecklist role={editingRole} value={editingPermissions} onChange={setEditingPermissions} />
            ) : (
              <span className="permission-summary">
                {user.role === "admin"
                  ? "Acesso total"
                  : USER_PERMISSION_OPTIONS
                      .filter((option) => user.permissions.includes(option.value))
                      .map((option) => option.label)
                      .join(", ") || "Sem permissoes"}
              </span>
            )}

            <span className="table-actions">
              {editingId === user.id ? (
                <>
                  <Button type="button" loading={saving} onClick={() => update(user)}>Salvar</Button>
                  <Button
                    type="button"
                    variant="secondary"
                    onClick={() => {
                      setEditingId(null);
                      setEditingPassword("");
                    }}
                  >
                    Cancelar
                  </Button>
                </>
              ) : (
                <>
                  <Button
                    type="button"
                    variant="secondary"
                    onClick={() => {
                      setEditingId(user.id);
                      setEditingUsername(user.username);
                      setEditingRole(user.role);
                      setEditingPermissions(user.permissions.length > 0 ? user.permissions : DEFAULT_OPERATOR_PERMISSIONS);
                      setEditingPassword("");
                    }}
                  >
                    Editar
                  </Button>
                  <Button type="button" variant="danger" disabled={user.id === currentUser.id} loading={saving} onClick={() => remove(user)}>
                    Excluir
                  </Button>
                </>
              )}
            </span>
          </div>
        ))}
      </section>
    </section>
  );
}
