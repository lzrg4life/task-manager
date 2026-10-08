import { api } from "./api.js";
import { initPage, show, ticketRow, ticketTable } from "./ticket-common.js";

await initPage();
const container = document.getElementById("list");
const kind = container.dataset.kind;
let page = 1;

async function load() {
  try {
    const data = await api(`/api/tickets/${kind}?page=${page}&per_page=25`);
    const pages = Math.max(1, Math.ceil(data.total / data.per_page));
    const prev = document.createElement("button");
    prev.textContent = "Previous";
    prev.className = "secondary";
    prev.disabled = page <= 1;
    prev.addEventListener("click", () => { page -= 1; load(); });
    const next = document.createElement("button");
    next.textContent = "Next";
    next.className = "secondary";
    next.disabled = page >= pages;
    next.addEventListener("click", () => { page += 1; load(); });
    const info = document.createElement("span");
    info.textContent = `Page ${page} of ${pages} (${data.total} tickets)`;
    const pager = document.createElement("div");
    pager.className = "pager";
    pager.append(prev, info, next);
    container.replaceChildren(ticketTable(data.items.map((t) => ticketRow(t))), pager);
    show("");
  } catch (err) { show(err.message); }
}

await load();
