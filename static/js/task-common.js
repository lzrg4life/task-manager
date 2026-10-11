import { initNavbar } from "./navbar.js?v=2";

export const STATUS_LABELS = {
  archived: "Archived", completed: "Completed", in_review: "In Review", active: "Active",
  up_next: "Up Next", backlog: "Backlog", removed: "Removed",
};
export const PRIORITY_LABELS = { normal: "Normal", high: "High", low: "Low" };

export async function initPage() {
  return initNavbar();
}

export function show(msg) {
  const el = document.getElementById("error");
  el.textContent = msg;
  el.hidden = !msg;
}

export function userName(u) { return u ? u.name : ""; }

export function td(text, className) {
  const el = document.createElement("td");
  el.textContent = text;
  if (className) el.className = className;
  return el;
}

export function taskRow(t, extra) {
  const tr = document.createElement("tr");
  if (t.priority === "high") tr.classList.add("priority-high");
  const link = document.createElement("a");
  link.href = `/task.html?id=${t.id}`;
  link.textContent = t.key;
  const idCell = document.createElement("td");
  idCell.append(link);
  const titleCell = td(t.title);
  if (t.blocked) {
    const blocked = document.createElement("span");
    blocked.className = "badge";
    blocked.textContent = "Blocked";
    blocked.title = t.blocked_reason;
    titleCell.append(" ", blocked);
  }
  for (const tag of t.tags || []) {
    const badge = document.createElement("span");
    badge.className = "badge";
    badge.textContent = `${tag.name}${tag.archived ? " (archived)" : ""}`;
    badge.title = tag.description;
    titleCell.append(" ", badge);
  }
  tr.append(idCell, titleCell, td(PRIORITY_LABELS[t.priority]),
    td(userName(t.reviewer)), td(userName(t.implementer)));
  if (extra) tr.append(extra);
  return tr;
}

export function taskTable(rows, extraHeader) {
  const table = document.createElement("table");
  const head = document.createElement("thead");
  const hr = document.createElement("tr");
  for (const h of ["ID", "Title", "Priority", "Reviewer", "Implementer", ...(extraHeader ? [""] : [])]) {
    const th = document.createElement("th");
    th.textContent = h;
    hr.append(th);
  }
  head.append(hr);
  const body = document.createElement("tbody");
  body.append(...rows);
  table.append(head, body);
  return table;
}
