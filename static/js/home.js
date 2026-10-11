import { api } from "./api.js";

const me = await api("/api/me");
if (me) location.replace("/tasks.html");
