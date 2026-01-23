import { create } from "zustand";

interface User {
  id: string;
  name: string;
  email: string;
}

interface AppState {
  user: User | null;
  isAuthenticated: boolean;
  theme: "light" | "dark";
  setUser: (user: User | null) => void;
  setTheme: (theme: "light" | "dark") => void;
  logout: () => void;
}

export const useAppStore = create<AppState>((set) => ({
  user: null,
  isAuthenticated: false,
  theme: "light",
  setUser: (user) => set({ user, isAuthenticated: !!user }),
  setTheme: (theme) => set({ theme }),
  logout: () => set({ user: null, isAuthenticated: false }),
}));
