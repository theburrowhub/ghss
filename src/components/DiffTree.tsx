import { useState } from "react";
import type { Category, SettingChange } from "../types";
import { CATEGORY_LABELS, CATEGORY_ORDER } from "../types";

interface Props {
  changes: SettingChange[];
  selectable: boolean;
  selected: Set<string>;
  onSelectedChange: (next: Set<string>) => void;
}

function fmt(v: unknown): string {
  if (v === null || v === undefined) return "—";
  if (typeof v === "boolean") return v ? "✓" : "✗";
  if (typeof v === "string") return v;
  return JSON.stringify(v).slice(0, 60);
}

export function DiffTree({ changes, selectable, selected, onSelectedChange }: Props) {
  const [collapsed, setCollapsed] = useState<Set<Category>>(new Set());
  const byCat = CATEGORY_ORDER.map((cat) => ({ cat, items: changes.filter((c) => c.category === cat) })).filter(
    (g) => g.items.length > 0,
  );

  const toggle = (key: string) => {
    const next = new Set(selected);
    next.has(key) ? next.delete(key) : next.add(key);
    onSelectedChange(next);
  };

  const toggleCat = (items: SettingChange[]) => {
    const applicables = items.filter((i) => i.applicable).map((i) => i.key);
    const allOn = applicables.every((k) => selected.has(k));
    const next = new Set(selected);
    applicables.forEach((k) => (allOn ? next.delete(k) : next.add(k)));
    onSelectedChange(next);
  };

  return (
    <div>
      {byCat.map(({ cat, items }) => {
        const applicables = items.filter((i) => i.applicable);
        const onCount = applicables.filter((i) => selected.has(i.key)).length;
        const isCollapsed = collapsed.has(cat);
        return (
          <div className="diff-cat" key={cat}>
            <div
              className="diff-cat-header"
              onClick={() =>
                setCollapsed((prev) => {
                  const next = new Set(prev);
                  next.has(cat) ? next.delete(cat) : next.add(cat);
                  return next;
                })
              }
            >
              <span>{isCollapsed ? "▶" : "▼"}</span>
              {selectable && applicables.length > 0 && (
                <input
                  type="checkbox"
                  aria-label={`Categoría ${CATEGORY_LABELS[cat]}`}
                  checked={onCount === applicables.length}
                  ref={(el) => {
                    if (el) el.indeterminate = onCount > 0 && onCount < applicables.length;
                  }}
                  onClick={(e) => e.stopPropagation()}
                  onChange={() => toggleCat(items)}
                />
              )}
              <strong>{CATEGORY_LABELS[cat]}</strong>
              <span className="muted">
                ({items.length} {items.length === 1 ? "cambio" : "cambios"})
              </span>
            </div>
            {!isCollapsed &&
              items.map((c) => (
                <div className={`diff-row${c.applicable ? "" : " na"}`} key={c.key}>
                  {selectable && (
                    <input
                      type="checkbox"
                      aria-label={c.label}
                      disabled={!c.applicable}
                      checked={c.applicable && selected.has(c.key)}
                      onChange={() => toggle(c.key)}
                    />
                  )}
                  <span style={{ flex: 1 }}>{c.label}</span>
                  {c.note && <span className="muted">{c.note}</span>}
                  <span className="val current">{fmt(c.current)}</span>
                  <span className="arrow">→</span>
                  <span className="val desired">{fmt(c.desired)}</span>
                </div>
              ))}
          </div>
        );
      })}
      {byCat.length === 0 && <p className="muted">Sin diferencias — el repo está en sync con la referencia. ✓</p>}
    </div>
  );
}
