import { useState } from "react";
import type { Category, SettingChange } from "../types";
import { CATEGORY_LABELS, CATEGORY_ORDER } from "../types";

interface Props {
  changes: SettingChange[];
  selectable: boolean;
  selected: Set<string>;
  onSelectedChange: (next: Set<string>) => void;
}

/** Renders a scalar value with a word + color based on its meaning (not its column). */
function ScalarValue({ v }: { v: unknown }) {
  if (typeof v === "boolean") {
    return <span className={`state ${v ? "on" : "off"}`}>{v ? "Enabled" : "Disabled"}</span>;
  }
  if (v === null || v === undefined) return <span className="state none">not set</span>;
  if (typeof v === "string") return <span className="state str">{v === "" ? "(empty)" : v}</span>;
  return <span className="state obj">object</span>;
}

const isScalar = (v: unknown) => typeof v !== "object" || v === null;

/** Explicit action label: makes clear what will happen to the target repo. */
function actionFor(c: SettingChange): { text: string; cls: string } {
  if (typeof c.desired === "boolean") {
    return c.desired
      ? { text: "Will be enabled", cls: "enable" }
      : { text: "Will be disabled", cls: "disable" };
  }
  if (c.current === null || c.current === undefined) return { text: "Will be created", cls: "create" };
  if (typeof c.desired === "object") return { text: "Will be updated", cls: "update" };
  return { text: "Will be changed", cls: "change" };
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
                  aria-label={`Category ${CATEGORY_LABELS[cat]}`}
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
                ({items.length} {items.length === 1 ? "change" : "changes"})
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
                  {isScalar(c.current) && isScalar(c.desired) && !(c.current === null && c.desired === null) && (
                    <span className="transition">
                      <ScalarValue v={c.current} />
                      <span className="arrow">→</span>
                      <ScalarValue v={c.desired} />
                    </span>
                  )}
                  {(() => {
                    const a = actionFor(c);
                    return <span className={`action ${a.cls}`}>{a.text}</span>;
                  })()}
                </div>
              ))}
          </div>
        );
      })}
      {byCat.length === 0 && <p className="muted">No differences — the repo is in sync with the reference. ✓</p>}
    </div>
  );
}
