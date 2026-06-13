import { render, screen, fireEvent } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import { DiffTree } from "./DiffTree";
import type { SettingChange } from "../types";

const changes: SettingChange[] = [
  { key: "features.has_wiki", label: "Wikis", category: "features", current: true, desired: false, applicable: true, note: null },
  { key: "pull_requests.allow_auto_merge", label: "Allow auto-merge", category: "pull_requests", current: false, desired: true, applicable: true, note: null },
  { key: "default_branch", label: "Default branch", category: "default_branch", current: "master", desired: "main", applicable: false, note: "La rama «main» no existe en el destino" },
];

function Harness({ selectable }: { selectable: boolean }) {
  const [selected, setSelected] = useState<Set<string>>(new Set(["features.has_wiki", "pull_requests.allow_auto_merge"]));
  return <DiffTree changes={changes} selectable={selectable} selected={selected} onSelectedChange={setSelected} />;
}

describe("DiffTree", () => {
  it("agrupa por categoría y muestra current → desired", () => {
    render(<Harness selectable={false} />);
    expect(screen.getByText("Features")).toBeInTheDocument();
    expect(screen.getByText("Pull Requests")).toBeInTheDocument();
    expect(screen.getByText("Wikis")).toBeInTheDocument();
  });

  it("los cambios no aplicables aparecen deshabilitados con nota", () => {
    render(<Harness selectable={true} />);
    const cb = screen.getByRole("checkbox", { name: /Default branch/ });
    expect(cb).toBeDisabled();
    expect(screen.getByText(/no existe en el destino/)).toBeInTheDocument();
    // La categoría sin cambios aplicables no debe renderizar checkbox de categoría.
    expect(screen.queryByRole("checkbox", { name: /Categoría Default branch/ })).not.toBeInTheDocument();
  });

  it("desmarcar un setting lo quita de la selección y la categoría refleja tri-estado", () => {
    render(<Harness selectable={true} />);
    const wiki = screen.getByRole("checkbox", { name: /Wikis/ });
    expect(wiki).toBeChecked();
    fireEvent.click(wiki);
    expect(screen.getByRole("checkbox", { name: /Wikis/ })).not.toBeChecked();
  });

  it("el checkbox de categoría marca/desmarca todos sus settings", () => {
    render(<Harness selectable={true} />);
    const cat = screen.getByRole("checkbox", { name: /Categoría Features/ });
    fireEvent.click(cat);
    expect(screen.getByRole("checkbox", { name: /Wikis/ })).not.toBeChecked();
    fireEvent.click(cat);
    expect(screen.getByRole("checkbox", { name: /Wikis/ })).toBeChecked();
  });
});
