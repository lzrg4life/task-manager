import { api } from "./api.js";
import { initPage, show, ticketRow, ticketTable, STATUS_LABELS } from "./ticket-common.js";

await initPage();
const board = document.getElementById("board");
const ticketSearch = document.getElementById("ticket-search");
const ticketAssignee = document.getElementById("ticket-assignee");
const ticketTags = document.getElementById("ticket-tags");
const ticketFilters = document.getElementById("ticket-filters");
const filterToggle = document.getElementById("toggle-filters");
const archiveDialog = document.getElementById("archive-dialog");
const archiveMessage = document.getElementById("archive-message");
const archiveError = document.getElementById("archive-error");
const archiveConfirm = document.getElementById("archive-confirm");
let ticketToArchive = null;
let ticketLists = [];
let assigneesLoaded = false;
let tagsLoaded = false;

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
ticketSearch.addEventListener("input", renderBoard);
ticketAssignee.addEventListener("change", renderBoard);
ticketTags.addEventListener("change", renderBoard);
filterToggle.addEventListener("click", () => {
  ticketFilters.hidden = !ticketFilters.hidden;
  filterToggle.setAttribute("aria-expanded", String(!ticketFilters.hidden));
  filterToggle.textContent = ticketFilters.hidden ? "Show filters" : "Hide filters";
});

function renderBoard() {
  const search = ticketSearch.value.trim().toLocaleLowerCase();
  const assigneeId = ticketAssignee.value;
  const tagIds = [...ticketTags.selectedOptions].map((option) => option.value);
  board.replaceChildren(...ticketLists.map((list, i) => {
    const section = document.createElement("section");
    section.className = "list";
    const tickets = list.tickets.filter((ticket) => {
      const matchesSearch = !search
        || ticket.key.toLocaleLowerCase().includes(search)
        || ticket.title.toLocaleLowerCase().includes(search);
      const matchesAssignee = !assigneeId
        || String(ticket.reviewer?.id) === assigneeId
        || String(ticket.implementer?.id) === assigneeId;
      const matchesTags = tagIds.every((id) =>
        ticket.tags.some((tag) => String(tag.id) === id));
      return matchesSearch && matchesAssignee && matchesTags;
    });
    const header = document.createElement("button");
    header.type = "button";
    header.className = "title-bar list-toggle";
    const collapsedByDefault = list.status !== "active";
    header.setAttribute("aria-expanded", String(!collapsedByDefault));
    const h = document.createElement("h2");
    h.textContent = `${STATUS_LABELS[list.status]} (${tickets.length})`;
    const body = document.createElement("div");
    body.classList.add("list-body");
    const setCollapsed = (collapsed) => {
      body.hidden = collapsed;
      header.setAttribute("aria-expanded", String(!collapsed));
    };
    header.addEventListener("click", () => setCollapsed(!body.hidden));
    header.append(h);
    section.append(header);
    setCollapsed(collapsedByDefault);
    if (tickets.length === 0) {
      const p = document.createElement("p");
      p.textContent = "No tickets.";
      body.append(p);
      section.append(body);
      return section;
    }
    const rows = tickets.map((t) => {
      const cell = document.createElement("td");
      cell.className = "actions";
      cell.append(
        list.status === "completed"
          ? moveButton("Archive", "Archive ticket", true, () => openArchiveDialog(t))
          : moveButton("↑", "Move up", i > 0, () => move(t.id, "up")),
        " ",
        moveButton("↓", "Move down", i < ticketLists.length - 1, () => move(t.id, "down")),
      );
      return ticketRow(t, cell);
    });
    body.append(ticketTable(rows, true));
    section.append(body);
    return section;
  }));
}

async function load() {
  try {
    if (!assigneesLoaded) {
      const users = await api("/api/users/assignable");
      for (const user of users) {
        const option = document.createElement("option");
        option.value = String(user.id);
        option.textContent = user.display_name
          ? `${user.display_name} (${user.email})`
          : user.email;
        ticketAssignee.append(option);
      }
      assigneesLoaded = true;
    }
    if (!tagsLoaded) {
      const tags = await api("/api/tags?include_archived=true");
      ticketTags.replaceChildren(...tags.map((tag) => {
        const option = document.createElement("option");
        option.value = String(tag.id);
        option.textContent = `${tag.name}${tag.archived ? " (archived)" : ""}`;
        option.title = tag.description;
        return option;
      }));
      tagsLoaded = true;
    }
    ticketLists = await api("/api/tickets");
    renderBoard();
    show("");
  } catch (err) { show(err.message); }
}

await load();
