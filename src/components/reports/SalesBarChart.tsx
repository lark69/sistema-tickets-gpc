import { Bar, BarChart, CartesianGrid, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";
import { formatCurrency } from "../../utils/currency";

interface SalesBarChartProps {
  data: Array<{ dateLabel: string; totalCents: number }>;
}

export function SalesBarChart({ data }: SalesBarChartProps) {
  const chartData = [...data]
    .slice(0, 7)
    .reverse()
    .map((d) => ({ dia: d.dateLabel.slice(5), total: d.totalCents / 100 }));

  if (chartData.length === 0) {
    return <p className="muted-text">Sem vendas para exibir no gráfico.</p>;
  }

  return (
    <div style={{ width: "100%", height: 280 }}>
      <ResponsiveContainer>
        <BarChart data={chartData} margin={{ top: 8, right: 8, bottom: 0, left: 8 }}>
          <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" />
          <XAxis dataKey="dia" fontSize={12} stroke="var(--text)" />
          <YAxis fontSize={12} stroke="var(--text)" tickFormatter={(v) => `R$ ${v}`} />
          <Tooltip
            formatter={(value: number) => formatCurrency(Math.round(value * 100))}
            labelFormatter={(label) => `Dia ${label}`}
          />
          <Bar dataKey="total" fill="var(--accent, #2563eb)" radius={[6, 6, 0, 0]} />
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}
