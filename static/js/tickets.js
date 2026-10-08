import { api } from "./api.js";
import { initPage, show, ticketRow, ticketTable, STATUS_LABELS } from "./ticket-common.js";

await initPage();
const board = document.getElementById("board");

async function move(id, direction) {
  try {
    await api(`/api/tickets/${id}/move`, { method: "POST", body: { direction } });
    await load();
  } catch (err) { show(err.message); }
}

function moveButton(label, title, enabled, handler) {
  const b = document.createElement("button");
  b.textContent = label;
  b.title = title;
  b.className = "secondary small";
  b.disabled = !enabled;
  b.addEventListener("click", handler);
  return b;
}

async function load() {
  try {
    const lists = await api("/api/tickets");
    board.replaceChildren(...lists.map((list, i) => {
      const section = document.createElement("section");
      section.className = "list";
      const h = document.createElement("h2");
      h.textContent = `${STATUS_LABELS[list.status]} (${list.tickets.length})`;
      section.append(h);
      if (list.tickets.length === 0) {
        const p = document.createElement("p");
        p.textContent = "No tickets.";
        section.append(p);
        return section;
      }
      const rows = list.tickets.map((t) => {
        const cell = document.createElement("td");
        cell.className = "actions";
        cell.append(
          moveButton("↑", "Move up", i > 0, () => move(t.id, "up")),
          " ",
          moveButton("↓", "Move down", i < lists.length - 1, () => move(t.id, "down")),
        );
        return ticketRow(t, cell);
      });
      section.append(ticketTable(rows, true));
      return section;
    }));
    show("");
  } catch (err) { show(err.message); }
}

await load();
