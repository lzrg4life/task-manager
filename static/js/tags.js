import { api } from "./api.js";
import { initPage } from "./ticket-common.js";

const me = await initPage();
if (!me.is_admin) {
  location.href = "/";
  throw new Error("not an admin");
}

const rows = document.getElementById("tag-rows");
const form = document.getElementById("tag-form");
const error = document.getElementById("error");

function showError(message) {
  error.textContent = message;
  error.hidden = !message;
}

async function load() {
  const tags = await api("/api/tags?include_archived=true");
  rows.replaceChildren(...tags.map((tag) => {
    const row = document.createElement("tr");
    const name = document.createElement("td");
    const description = document.createElement("td");
    name.textContent = tag.name;
    description.textContent = tag.description;
    const status = document.createElement("td");
    status.textContent = tag.archived ? "Archived" : "Active";
    const action = document.createElement("td");
    if (!tag.archived) {
      const archive = document.createElement("button");
      archive.type = "button";
      archive.className = "secondary small";
      archive.textContent = "Archive";
      archive.addEventListener("click", async () => {
        archive.disabled = true;
        try {
          await api(`/api/tags/${tag.id}/archive`, { method: "POST" });
          await load();
          showError("");
        } catch (err) {
          archive.disabled = false;
          showError(err.message);
        }
      });
      action.append(archive);
    }
    row.append(name, description, status, action);
    return row;
  }));
}

form.addEventListener("submit", async (event) => {
  event.preventDefault();
  showError("");
  try {
    await api("/api/tags", {
      method: "POST",
      body: {
        name: form.elements.name.value,
        description: form.elements.description.value,
      },
    });
    form.reset();
    await load();
  } catch (err) {
    showError(err.message);
  }
});

try {
  await load();
} catch (err) {
  showError(err.message);
}
