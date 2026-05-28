import { ChevronLeft, ChevronRight, Palette, Printer, Tags, Ticket } from "lucide-react";
import { useState } from "react";
import { Button } from "../components/ui/Button";

const slides = [
  {
    icon: Ticket,
    title: "Bem-vindo ao Sistema de Tickets GPC",
    text: "Organize produtos e imprima tickets térmicos com uma experiência simples, rápida e profissional."
  },
  {
    icon: Tags,
    title: "Cadastro de produtos",
    text: "Crie, edite, pesquise e mantenha sua lista de produtos pronta para a rotina do comércio."
  },
  {
    icon: Printer,
    title: "Impressão de tickets",
    text: "Gere tickets com empresa, CPF/CNPJ, valor, validade e mensagem de agradecimento."
  },
  {
    icon: Palette,
    title: "Personalização do sistema",
    text: "Ajuste tema, dados da empresa e impressora para deixar tudo alinhado ao seu atendimento."
  }
];

interface OnboardingPageProps {
  saving: boolean;
  onFinish: () => Promise<void>;
}

export function OnboardingPage({ saving, onFinish }: OnboardingPageProps) {
  const [index, setIndex] = useState(0);
  const slide = slides[index];
  const Icon = slide.icon;
  const isFirst = index === 0;
  const isLast = index === slides.length - 1;

  async function handleNext() {
    if (isLast) {
      await onFinish();
      return;
    }

    setIndex((current) => current + 1);
  }

  return (
    <main className="onboarding-screen">
      <section className="onboarding-panel">
        <div className="onboarding-brand">
          <span className="brand-symbol">GPC</span>
          <strong>Sistema de Tickets GPC</strong>
        </div>

        <div className="slide-frame" key={slide.title}>
          <div className="slide-icon">
            <Icon size={42} />
          </div>
          <span className="slide-counter">
            {index + 1} de {slides.length}
          </span>
          <h1>{slide.title}</h1>
          <p>{slide.text}</p>
        </div>

        <div className="slide-dots" aria-label="Progresso do onboarding">
          {slides.map((item, slideIndex) => (
            <button
              key={item.title}
              type="button"
              className={slideIndex === index ? "active" : ""}
              aria-label={`Ir para slide ${slideIndex + 1}`}
              onClick={() => setIndex(slideIndex)}
            />
          ))}
        </div>

        <div className="onboarding-actions">
          {!isFirst ? (
            <Button
              type="button"
              variant="secondary"
              icon={<ChevronLeft size={18} />}
              onClick={() => setIndex((current) => current - 1)}
            >
              Back
            </Button>
          ) : (
            <span />
          )}
          <Button
            type="button"
            icon={<ChevronRight size={18} />}
            loading={saving}
            onClick={handleNext}
          >
            {isLast ? "Finalizar" : "Next"}
          </Button>
        </div>
      </section>
    </main>
  );
}
