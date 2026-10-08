import { login, saveSession } from "./api.js";

const form = document.getElementById("login-form");
const error = document.getElementById("error");

form.addEventListener("submit", async (e) => {
  e.preventDefault();
  error.hidden = true;
  const data = new FormData(form);
  try {
    const result = await login(data.get("email"), data.get("password"));
    saveSession(result);
    location.href = result.user.roles.includes("admin") ? "/users.html" : "/";
  } catch (err) {
    error.textContent = err.message;
    error.hidden = false;
  }
});
