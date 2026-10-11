import { api } from "./api.js";
import { initPage, show, taskRow, taskTable, STATUS_LABELS } from "./task-common.js";

await initPage();
const board = document.getElementById("board");
const taskSearch = document.getElementById("task-search");
const taskAssignee = document.getElementById("task-assignee");
const taskTags = document.getElementById("task-tags");
const taskFilters = document.getElementById("task-filters");
const filterToggle = document.getElementById("toggle-filters");
const archiveDialog = document.getElementById("archive-dialog");
const archiveMessage = document.getElementById("archive-message");
const archiveError = document.getElementById("archive-error");
const archiveConfirm = document.getElementById("archive-confirm");
let taskToArchive = null;
let taskLists = [];
let assigneesLoaded = false;
let tagsLoaded = false;
const expandedLists = new Map();
async function move(id, direction) {
  try {
    await api(`/api/tasks/${id}/move`, { method: "POST", body: { direction } });
    await load();
  } catch (err) { show(err.message); }
}

function openArchiveDialog(task) {
  taskToArchive = task;
  archiveMessage.textContent = `Are you sure you want to archive "${task.title}"?`;
  archiveError.textContent = "";
  archiveError.hidden = true;
  archiveConfirm.disabled = false;
  archiveDialog.showModal();
}

async function archiveTask() {
  if (!taskToArchive) return;
  archiveConfirm.disabled = true;
  try {
    await api(`/api/tasks/${taskToArchive.id}`, {
      method: "PATCH",
      body: { status: "archived" },
    });
    archiveDialog.close();
    taskToArchive = null;
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
  taskToArchive = null;
});
archiveConfirm.addEventListener("click", archiveTask);
taskSearch.addEventListener("input", renderBoard);
taskAssignee.addEventListener("change", renderBoard);
taskTags.addEventListener("change", renderBoard);
filterToggle.addEventListener("click", () => {
  taskFilters.hidden = !taskFilters.hidden;
  filterToggle.setAttribute("aria-expanded", String(!taskFilters.hidden));
  filterToggle.textContent = taskFilters.hidden ? "Show filters" : "Hide filters";
});

function renderBoard() {
  const search = taskSearch.value.trim().toLocaleLowerCase();
  const assigneeId = taskAssignee.value;
  const tagIds = [...taskTags.selectedOptions].map((option) => option.value);
  board.replaceChildren(...taskLists.map((list, i) => {
    const section = document.createElement("section");
    section.className = "list";
    const tasks = list.tasks.filter((task) => {
      const matchesSearch = !search
        || task.key.toLocaleLowerCase().includes(search)
        || task.title.toLocaleLowerCase().includes(search);
      const matchesAssignee = !assigneeId
        || String(task.reviewer?.id) === assigneeId
        || String(task.implementer?.id) === assigneeId;
      const matchesTags = tagIds.every((id) =>
        task.tags.some((tag) => String(tag.id) === id));
      return matchesSearch && matchesAssignee && matchesTags;
    });
    const header = document.createElement("button");
    header.type = "button";
    header.className = "title-bar list-toggle";
    const expanded = expandedLists.get(list.status) ?? list.status === "active";
    const h = document.createElement("h2");
    h.textContent = `${STATUS_LABELS[list.status]} (${tasks.length})`;
    const body = document.createElement("div");
    body.classList.add("list-body");
    const setCollapsed = (collapsed) => {
      body.hidden = collapsed;
      header.setAttribute("aria-expanded", String(!collapsed));
      expandedLists.set(list.status, !collapsed);
    };
    header.addEventListener("click", () => setCollapsed(!body.hidden));
    header.append(h);
    section.append(header);
    setCollapsed(!expanded);
    if (tasks.length === 0) {
      const p = document.createElement("p");
      p.textContent = "No tasks.";
      body.append(p);
      section.append(body);
      return section;
    }
    const rows = tasks.map((t) => {
      const cell = document.createElement("td");
      cell.className = "actions";
      cell.append(
        list.status === "completed"
          ? moveButton("Archive", "Archive task", true, () => openArchiveDialog(t))
          : moveButton("↑", "Move up", i > 0, () => move(t.id, "up")),
        " ",
        moveButton("↓", "Move down", i < taskLists.length - 1, () => move(t.id, "down")),
      );
      return taskRow(t, cell);
    });
    body.append(taskTable(rows, true));
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
        taskAssignee.append(option);
      }
      assigneesLoaded = true;
    }
    if (!tagsLoaded) {
      const tags = await api("/api/tags?include_archived=true");
      taskTags.replaceChildren(...tags.map((tag) => {
        const option = document.createElement("option");
        option.value = String(tag.id);
        option.textContent = `${tag.name}${tag.archived ? " (archived)" : ""}`;
        option.title = tag.description;
        return option;
      }));
      tagsLoaded = true;
    }
    taskLists = await api("/api/tasks");
    renderBoard();
    show("");
  } catch (err) { show(err.message); }
}

await load();
