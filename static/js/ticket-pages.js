import { api } from "./api.js";
import { initPage, show, ticketRow, ticketTable } from "./ticket-common.js";

await initPage();
const container = document.getElementById("list");
const kind = container.dataset.kind;
const ticketSearch = document.getElementById("ticket-search");
const ticketAssignee = document.getElementById("ticket-assignee");
const ticketTags = document.getElementById("ticket-tags");
const ticketFilters = document.getElementById("ticket-filters");
const filterToggle = document.getElementById("toggle-filters");
let page = 1;
let requestId = 0;
let assigneesLoaded = false;
let assigneesPromise;
let tagsLoaded = false;
let tagsPromise;
let searchTimeout;

ticketSearch.addEventListener("input", () => {
  clearTimeout(searchTimeout);
  searchTimeout = setTimeout(() => {
    page = 1;
    load();
  }, 200);
});
ticketAssignee.addEventListener("change", () => {
  page = 1;
  load();
});
ticketTags.addEventListener("change", () => {
  page = 1;
  load();
});
filterToggle.addEventListener("click", () => {
  ticketFilters.hidden = !ticketFilters.hidden;
  filterToggle.setAttribute("aria-expanded", String(!ticketFilters.hidden));
  filterToggle.textContent = ticketFilters.hidden ? "Show filters" : "Hide filters";
});

async function load() {
  const currentRequest = ++requestId;
  try {
    if (!assigneesLoaded) {
      assigneesPromise ??= api("/api/users/assignable")
        .then((users) => {
          for (const user of users) {
            const option = document.createElement("option");
            option.value = String(user.id);
            option.textContent = user.display_name
              ? `${user.display_name} (${user.email})`
              : user.email;
            ticketAssignee.append(option);
          }
          assigneesLoaded = true;
        })
        .catch((err) => {
          assigneesPromise = undefined;
          throw err;
        });
      await assigneesPromise;
      if (currentRequest !== requestId) return;
    }
    if (!tagsLoaded) {
      tagsPromise ??= api("/api/tags?include_archived=true")
        .then((tags) => {
          ticketTags.replaceChildren(...tags.map((tag) => {
            const option = document.createElement("option");
            option.value = String(tag.id);
            option.textContent = `${tag.name}${tag.archived ? " (archived)" : ""}`;
            option.title = tag.description;
            return option;
          }));
          tagsLoaded = true;
        })
        .catch((err) => {
          tagsPromise = undefined;
          throw err;
        });
      await tagsPromise;
      if (currentRequest !== requestId) return;
    }
    const params = new URLSearchParams({ page: String(page), per_page: "25" });
    const search = ticketSearch.value.trim();
    if (search) params.set("search", search);
    if (ticketAssignee.value) params.set("assignee_id", ticketAssignee.value);
    const tagIds = [...ticketTags.selectedOptions].map((option) => option.value);
    if (tagIds.length) params.set("tag_ids", tagIds.join(","));
    const data = await api(`/api/tickets/${kind}?${params}`);
    if (currentRequest !== requestId) return;
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
  } catch (err) {
    if (currentRequest === requestId) show(err.message);
  }
}

await load();
