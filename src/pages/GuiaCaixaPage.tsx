import {
  ArrowLeft,
  CalendarClock,
  ClipboardCheck,
  Lock,
  Lightbulb,
  Minus,
  PlayCircle,
  Plus,
  ShoppingCart,
  StopCircle
} from "lucide-react";
import type { ReactNode } from "react";
import { Button } from "../components/ui/Button";

interface GuiaCaixaPageProps {
  onBack: () => void;
}

interface Passo {
  numero: number;
  titulo: string;
  local: string;
  icon: ReactNode;
  descricao: ReactNode;
}

const PASSOS: Passo[] = [
  {
    numero: 1,
    titulo: "Abrir o turno",
    local: "menu Fechamento",
    icon: <PlayCircle size={22} />,
    descricao: (
      <>
        Informe o <strong>fundo de troco</strong> (o dinheiro que já está na gaveta) e clique em
        <strong> Abrir turno</strong>. A partir daqui o caixa libera as vendas.
      </>
    )
  },
  {
    numero: 2,
    titulo: "Vender",
    local: "menu Caixa e Mesas",
    icon: <ShoppingCart size={22} />,
    descricao: (
      <>
        <strong>Caixa</strong>: venda direta, o cliente leva na hora.{" "}
        <strong>Mesas</strong>: comanda aberta, fecha quando o cliente vai embora. O sistema soma o
        dinheiro sozinho.
      </>
    )
  },
  {
    numero: 3,
    titulo: "Sangria e Suprimento",
    local: "menu Caixa",
    icon: (
      <span className="guia-dupla-icon">
        <Minus size={18} />
        <Plus size={18} />
      </span>
    ),
    descricao: (
      <>
        <strong>Sangria</strong> = tirar dinheiro da gaveta.{" "}
        <strong>Suprimento</strong> = colocar dinheiro na gaveta. Use só quando precisar.
      </>
    )
  },
  {
    numero: 4,
    titulo: "Fechar o turno",
    local: "menu Fechamento",
    icon: <StopCircle size={22} />,
    descricao: (
      <>
        Conte o dinheiro da gaveta e digite no campo <strong>Dinheiro físico contado</strong>. O
        sistema mostra o esperado e a diferença. Havendo diferença, escreva uma observação.
      </>
    )
  },
  {
    numero: 5,
    titulo: "Consolidar o período",
    local: "menu Fechamento, fim do dia",
    icon: <ClipboardCheck size={22} />,
    descricao: (
      <>
        Com <strong>todos os turnos do dia fechados</strong>, clique em
        <strong> Consolidar período</strong> para fechar o dia e somar tudo nos relatórios.
      </>
    )
  },
  {
    numero: 6,
    titulo: "Bloquear o período",
    local: "só Administrador (opcional)",
    icon: <Lock size={22} />,
    descricao: (
      <>
        <strong>Bloquear</strong> sela o dia para auditoria: nenhuma venda daquele dia pode mais ser
        alterada. Use quando tiver certeza de que está tudo certo.
      </>
    )
  }
];

export function GuiaCaixaPage({ onBack }: GuiaCaixaPageProps) {
  return (
    <section className="page-stack">
      <div className="dashboard-header">
        <div className="section-heading">
          <span>Guia rápido</span>
          <h1>Como usar o caixa</h1>
          <p>Tudo gira em torno do TURNO: abra, venda e, no fim, conte a gaveta para fechar.</p>
        </div>
        <div className="cash-register-pill">
          <CalendarClock size={18} />
          Passo a passo
        </div>
      </div>

      {/* Resumo: os 4 estágios */}
      <section className="settings-section">
        <h2>O ciclo do caixa</h2>
        <div className="guia-fluxo">
          <span className="guia-fluxo-item">Turno aberto<small>vendendo</small></span>
          <span className="guia-fluxo-seta">→</span>
          <span className="guia-fluxo-item">Turno fechado<small>gaveta contada</small></span>
          <span className="guia-fluxo-seta">→</span>
          <span className="guia-fluxo-item">Período consolidado<small>dia somado</small></span>
          <span className="guia-fluxo-seta">→</span>
          <span className="guia-fluxo-item">Período bloqueado<small>selado</small></span>
        </div>
        <p className="muted-text">Cada etapa "tranca" mais que a anterior. Sem turno aberto, o caixa não deixa vender.</p>
      </section>

      {/* Passos numerados */}
      <section className="settings-section">
        <h2>Passo a passo do dia</h2>
        <div className="guia-passos">
          {PASSOS.map((passo) => (
            <div className="guia-passo" key={passo.numero}>
              <div className="guia-passo-num">{passo.numero}</div>
              <div className="guia-passo-icon">{passo.icon}</div>
              <div className="guia-passo-texto">
                <h3>
                  {passo.titulo} <span className="guia-passo-local">{passo.local}</span>
                </h3>
                <p>{passo.descricao}</p>
              </div>
            </div>
          ))}
        </div>
      </section>

      {/* Como o esperado é calculado */}
      <section className="settings-section">
        <h2>Como o "esperado na gaveta" é calculado</h2>
        <div className="guia-formula">
          Fundo de troco <span>+</span> Vendas em dinheiro <span>+</span> Suprimentos <span>−</span> Sangrias
        </div>
        <p className="muted-text">Cartão e PIX não entram na gaveta de dinheiro — só o que é em espécie.</p>
      </section>

      {/* Dicas */}
      <section className="settings-section">
        <h2><Lightbulb size={18} /> Boas a saber</h2>
        <ul className="guia-dicas">
          <li><strong>Trocar de operador?</strong> Feche o turno atual e abra outro. Vários turnos por dia são normais.</li>
          <li><strong>Esqueceu de abrir o turno?</strong> O sistema avisa e bloqueia a venda. Abra em Fechamento e siga.</li>
          <li><strong>Bar que vira a madrugada?</strong> Em Fechamento → Dia fiscal deslocado, defina a hora de virada (ex.: 06:00). Vendas da madrugada contam no dia anterior.</li>
          <li><strong>Onde abro/fecho o caixa?</strong> Sempre em <strong>Fechamento</strong>. O <strong>Caixa</strong> é só para vender e fazer sangria/suprimento.</li>
        </ul>
      </section>

      <div className="form-actions">
        <Button type="button" variant="secondary" icon={<ArrowLeft size={18} />} onClick={onBack}>
          Voltar
        </Button>
      </div>
    </section>
  );
}
