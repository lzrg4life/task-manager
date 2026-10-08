// Token handling and fetch wrapper. Access token + expiry live in sessionStorage (per tab);
// the refresh token lives in localStorage and is only ever sent to /api/auth/refresh and /logout.
const ACCESS = "access_token";
const EXPIRES = "access_expires_at";
const REFRESH = "refresh_token";

export function saveSession(data) {
  sessionStorage.setItem(ACCESS, data.access_token);
  sessionStorage.setItem(EXPIRES, String(Date.now() + data.expires_in * 1000));
  localStorage.setItem(REFRESH, data.refresh_token);
}

export function clearSession() {
  sessionStorage.removeItem(ACCESS);
  sessionStorage.removeItem(EXPIRES);
  localStorage.removeItem(REFRESH);
}

export function redirectToLogin() {
  clearSession();
  location.href = "/login.html";
}

async function postJson(path, body) {
  const res = await fetch(path, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  const data = res.status === 204 ? null : await res.json().catch(() => null);
  if (!res.ok) throw new Error((data && data.error) || `Request failed (${res.status})`);
  return data;
}

export const login = (email, password) => postJson("/api/auth/login", { email, password });

let refreshing = null;
async function refreshSession() {
  const token = localStorage.getItem(REFRESH);
  if (!token) return false;
  // Share one in-flight refresh so rotation is not triggered twice by parallel requests.
  refreshing ??= postJson("/api/auth/refresh", { refresh_token: token })
    .then((d) => { saveSession(d); return true; })
    .catch(() => false)
    .finally(() => { refreshing = null; });
  return refreshing;
}

async function validAccessToken() {
  const token = sessionStorage.getItem(ACCESS);
  const expires = Number(sessionStorage.getItem(EXPIRES) || 0);
  if (token && Date.now() < expires - 10000) return token;
  return (await refreshSession()) ? sessionStorage.getItem(ACCESS) : null;
}

export async function api(path, { method = "GET", body } = {}) {
  for (let attempt = 0; attempt < 2; attempt++) {
    const token = attempt === 0 ? await validAccessToken() : (await refreshSession()) && sessionStorage.getItem(ACCESS);
    if (!token) return redirectToLogin();
    const res = await fetch(path, {
      method,
      headers: { Authorization: `Bearer ${token}`, ...(body ? { "Content-Type": "application/json" } : {}) },
      body: body ? JSON.stringify(body) : undefined,
    });
    if (res.status === 401 && attempt === 0) continue;
    if (res.status === 401) return redirectToLogin();
    const data = res.status === 204 ? null : await res.json().catch(() => null);
    if (!res.ok) throw new Error((data && data.error) || `Request failed (${res.status})`);
    return data;
  }
}

export async function logout() {
  const token = localStorage.getItem(REFRESH);
  if (token) await postJson("/api/auth/logout", { refresh_token: token }).catch(() => {});
  redirectToLogin();
}
