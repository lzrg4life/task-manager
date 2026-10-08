import { api } from "./api.js";
import { initPage, show, ticketRow, ticketTable, STATUS_LABELS } from "./ticket-common.js";

await initPage();
const board = document.getElementById("board");
const archiveDialog = document.getElementById("archive-dialog");
const archiveMessage = document.getElementById("archive-message");
const archiveError = document.getElementById("archive-error");
const archiveConfirm = document.getElementById("archive-confirm");
let ticketToArchive = null;

async function move(id, direction) {
  try {
    await api(`/api/tickets/${id}/move`, { method: "POST", body: { direction } });
    await load();
  } catch (err) { show(err.message); }
}

function openArchiveDialog(ticket) {
  ticketToArchive = ticket;
  archiveMessage.textContent = `Are you sure you want to archive "${ticket.title}"?`;
  archiveError.textContent = "";
  archiveError.hidden = true;
  archiveConfirm.disabled = false;
  archiveDialog.showModal();
}

async function archiveTicket() {
  if (!ticketToArchive) return;
  archiveConfirm.disabled = true;
  try {
    await api(`/api/tickets/${ticketToArchive.id}`, {
      method: "PATCH",
      body: { status: "archived" },
    });
    archiveDialog.close();
    ticketToArchive = null;
    await load();
  } catch (err) {
    archiveError.textContent = err.message;
    archiveError.hidden = false;
  } finally {
    archiveConfirm.disabled = false;
  }
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

document.getElementById("archive-cancel").addEventListener("click", () => {
  archiveDialog.close();
  ticketToArchive = null;
});
archiveConfirm.addEventListener("click", archiveTicket);

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
          list.status === "completed"
            ? moveButton("Archive", "Archive ticket", true, () => openArchiveDialog(t))
            : moveButton("↑", "Move up", i > 0, () => move(t.id, "up")),
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
