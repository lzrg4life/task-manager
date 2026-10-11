import { api } from "./api.js";
import { initPage, show, taskRow, taskTable, STATUS_LABELS } from "./task-common.js";

const currentUser = await initPage();
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
const openComments = new Set();
const commentsByTask = new Map();
const loadingComments = new Set();
const commentsErrors = new Map();
const editingComments = new Map();

function setCommentsError(taskId, message) {
  commentsErrors.set(taskId, message);
}

async function loadComments(taskId) {
  loadingComments.add(taskId);
  commentsErrors.delete(taskId);
  renderBoard();
  try {
    commentsByTask.set(taskId, await api(`/api/tasks/${taskId}/comments`));
  } catch (err) {
    setCommentsError(taskId, err.message);
  } finally {
    loadingComments.delete(taskId);
    renderBoard();
  }
}

function commentActionButton(label, handler) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "secondary small";
  button.textContent = label;
  button.addEventListener("click", handler);
  return button;
}

function renderComments(taskId) {
  const thread = document.createElement("div");
  thread.className = "comment-thread";
  if (loadingComments.has(taskId)) {
    const loading = document.createElement("p");
    loading.textContent = "Loading comments…";
    thread.append(loading);
  } else if (commentsErrors.has(taskId)) {
    const error = document.createElement("p");
    error.className = "error";
    error.textContent = commentsErrors.get(taskId);
    thread.append(error, commentActionButton("Retry", () => loadComments(taskId)));
  } else {
    for (const comment of commentsByTask.get(taskId) || []) {
      const item = document.createElement("article");
      item.className = "comment";
      const meta = document.createElement("div");
      meta.className = "comment-meta";
      meta.textContent = `${comment.author_name} · ${comment.created_at}${comment.updated_at !== comment.created_at && !comment.deleted_at ? " (edited)" : ""}`;
      item.append(meta);

      if (comment.deleted_at) {
        const deleted = document.createElement("p");
        deleted.className = "comment-body";
        deleted.textContent = "Comment deleted";
        item.append(deleted);
      } else if (editingComments.get(taskId) === comment.id) {
        const form = document.createElement("form");
        form.className = "comment-form";
        const textarea = document.createElement("textarea");
        textarea.maxLength = 5000;
        textarea.required = true;
        textarea.value = comment.body;
        const actions = document.createElement("div");
        actions.className = "comment-actions";
        const save = document.createElement("button");
        save.type = "submit";
        save.textContent = "Save";
        actions.append(save, commentActionButton("Cancel", () => {
          editingComments.delete(taskId);
          renderBoard();
        }));
        form.append(textarea, actions);
        form.addEventListener("submit", async (event) => {
          event.preventDefault();
          save.disabled = true;
          try {
            await api(`/api/tasks/${taskId}/comments/${comment.id}`, {
              method: "PATCH",
              body: { body: textarea.value },
            });
            editingComments.delete(taskId);
            await loadComments(taskId);
          } catch (err) {
            setCommentsError(taskId, err.message);
            renderBoard();
          }
        });
        item.append(form);
      } else {
        const body = document.createElement("p");
        body.className = "comment-body";
        body.textContent = comment.body;
        item.append(body);
        if (comment.user_id === currentUser.id || currentUser.is_admin) {
          const actions = document.createElement("div");
          actions.className = "comment-actions";
          actions.append(commentActionButton("Edit", () => {
            editingComments.set(taskId, comment.id);
            renderBoard();
          }));
          actions.append(commentActionButton("Delete", async () => {
            if (!confirm("Delete this comment?")) return;
            try {
              await api(`/api/tasks/${taskId}/comments/${comment.id}`, { method: "DELETE" });
              await loadComments(taskId);
            } catch (err) {
              setCommentsError(taskId, err.message);
              renderBoard();
            }
          }));
          item.append(actions);
        }
      }
      thread.append(item);
    }

    const form = document.createElement("form");
    form.className = "comment-form";
    const textarea = document.createElement("textarea");
    textarea.maxLength = 5000;
    textarea.required = true;
    textarea.placeholder = "Add a comment…";
    const submit = document.createElement("button");
    submit.type = "submit";
    submit.textContent = "Add comment";
    form.append(textarea, submit);
    form.addEventListener("submit", async (event) => {
      event.preventDefault();
      submit.disabled = true;
      try {
        await api(`/api/tasks/${taskId}/comments`, {
          method: "POST",
          body: { body: textarea.value },
        });
        await loadComments(taskId);
      } catch (err) {
        setCommentsError(taskId, err.message);
        renderBoard();
      }
    });
    thread.append(form);
  }
  return thread;
}

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
    const collapsedByDefault = list.status !== "active";
    header.setAttribute("aria-expanded", String(!collapsedByDefault));
    const h = document.createElement("h2");
    h.textContent = `${STATUS_LABELS[list.status]} (${tasks.length})`;
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
    if (tasks.length === 0) {
      const p = document.createElement("p");
      p.textContent = "No tasks.";
      body.append(p);
      section.append(body);
      return section;
    }
    const rows = tasks.flatMap((t) => {
      const cell = document.createElement("td");
      cell.className = "actions";
      cell.append(
        list.status === "completed"
          ? moveButton("Archive", "Archive task", true, () => openArchiveDialog(t))
          : moveButton("↑", "Move up", i > 0, () => move(t.id, "up")),
        " ",
        moveButton("↓", "Move down", i < taskLists.length - 1, () => move(t.id, "down")),
      );
      const comments = commentActionButton(
        openComments.has(t.id)
          ? "Hide comments"
          : `Comments${commentsByTask.has(t.id) ? ` (${commentsByTask.get(t.id).length})` : ""}`,
        () => {
          if (openComments.has(t.id)) {
            openComments.delete(t.id);
          } else {
            openComments.add(t.id);
            if (!commentsByTask.has(t.id)) void loadComments(t.id);
          }
          renderBoard();
        },
      );
      cell.append(" ", comments);
      const row = taskRow(t, cell);
      if (!openComments.has(t.id)) return [row];
      const commentsRow = document.createElement("tr");
      commentsRow.className = "comments-row";
      const commentsCell = document.createElement("td");
      commentsCell.colSpan = 6;
      commentsCell.append(renderComments(t.id));
      commentsRow.append(commentsCell);
      return [row, commentsRow];
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
