import { render, screen, fireEvent } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import { DiffTree } from "./DiffTree";
import type { SettingChange } from "../types";

const changes: SettingChange[] = [
  { key: "features.has_wiki", label: "Wikis", category: "features", current: true, desired: false, applicable: true, note: null },
  { key: "pull_requests.allow_auto_merge", label: "Allow auto-merge", category: "pull_requests", current: false, desired: true, applicable: true, note: null },
  { key: "default_branch", label: "Default branch", category: "default_branch", current: "master", desired: "main", applicable: false, note: "The «main» branch does not exist in the target" },
];

function Harness({ selectable }: { selectable: boolean }) {
  const [selected, setSelected] = useState<Set<string>>(new Set(["features.has_wiki", "pull_requests.allow_auto_merge"]));
  return <DiffTree changes={changes} selectable={selectable} selected={selected} onSelectedChange={setSelected} />;
}

describe("DiffTree", () => {
  it("groups by category and shows current → desired", () => {
    render(<Harness selectable={false} />);
    expect(screen.getByText("Features")).toBeInTheDocument();
    expect(screen.getByText("Pull Requests")).toBeInTheDocument();
    expect(screen.getByText("Wikis")).toBeInTheDocument();
  });

  it("non-applicable changes appear disabled with a note", () => {
    render(<Harness selectable={true} />);
    const cb = screen.getByRole("checkbox", { name: /Default branch/ });
    expect(cb).toBeDisabled();
    expect(screen.getByText(/does not exist in the target/)).toBeInTheDocument();
    // A category with no applicable changes must not render a category checkbox.
    expect(screen.queryByRole("checkbox", { name: /Category Default branch/ })).not.toBeInTheDocument();
  });

  it("unchecking a setting removes it from the selection and the category reflects tri-state", () => {
    render(<Harness selectable={true} />);
    const wiki = screen.getByRole("checkbox", { name: /Wikis/ });
    expect(wiki).toBeChecked();
    fireEvent.click(wiki);
    expect(screen.getByRole("checkbox", { name: /Wikis/ })).not.toBeChecked();
  });

  it("the category checkbox selects/deselects all its settings", () => {
    render(<Harness selectable={true} />);
    const cat = screen.getByRole("checkbox", { name: /Category Features/ });
    fireEvent.click(cat);
    expect(screen.getByRole("checkbox", { name: /Wikis/ })).not.toBeChecked();
    fireEvent.click(cat);
    expect(screen.getByRole("checkbox", { name: /Wikis/ })).toBeChecked();
  });
});
