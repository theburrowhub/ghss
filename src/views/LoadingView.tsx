interface Props {
  title: string;
  detail?: string;
}

export function LoadingView({ title, detail }: Props) {
  return (
    <div className="view" style={{ display: "grid", placeItems: "center" }}>
      <div className="card loading-card">
        <div className="spinner" />
        <div>
          <div style={{ fontWeight: 600 }}>{title}</div>
          {detail && <div className="muted" style={{ marginTop: 4 }}>{detail}</div>}
        </div>
      </div>
    </div>
  );
}
