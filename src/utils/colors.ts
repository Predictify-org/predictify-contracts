type Theme = 'light' | 'dark';

type ColorScheme = {
  background: string;
  card: string;
  primary: string;
  secondary: string;
  accent: string;
};

const lightColors: ColorScheme = {
  background: 'hsl(220, 20%, 95%)', // Cool gray with slight blue tint
  card: 'hsl(0, 0%, 100%)', // Pure white
  primary: 'hsl(220, 85%, 57%)', // Deep blue
  secondary: 'hsl(190, 80%, 50%)', // Teal
  accent: 'hsl(250, 70%, 65%)', // Light purple
};

const darkColors: ColorScheme = {
  background: 'hsl(220, 25%, 15%)', // Deep navy
  card: 'hsl(220, 25%, 20%)', // Slightly lighter navy
  primary: 'hsl(220, 85%, 60%)', // Bright blue
  secondary: 'hsl(190, 80%, 55%)', // Bright teal
  accent: 'hsl(250, 70%, 70%)', // Bright purple
};

export const colors = {
  light: lightColors,
  dark: darkColors,
};

/**
 * Get color scheme based on theme
 * @param theme - 'light' or 'dark'
 * @returns ColorScheme object
 */
export const getColors = (theme: Theme): ColorScheme => {
  return colors[theme];
};

/**
 * Get a specific color by key and theme
 * @param key - Color key (background, card, primary, secondary, accent)
 * @param theme - 'light' or 'dark'
 * @returns HSL color string
 */
export const getColor = (key: keyof ColorScheme, theme: Theme): string => {
  return colors[theme][key];
};

export type { ColorScheme, Theme };
