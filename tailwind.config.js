/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./App.{js,jsx,ts,tsx}",
    "./app/**/*.{js,jsx,ts,tsx}",
    "./src/**/*.{js,jsx,ts,tsx}",
  ],
  presets: [require("nativewind/preset")],
  theme: {
<<<<<<< HEAD
    extend: {
      colors: {
        primary: {
          light: '#19c3e6',
          DEFAULT: '#19c3e6',
          dark: '#0099b3',
        },
        gradient: {
          start: '#20afe7',
          mid: '#2c8aec',
          end: '#586ce9',
        },
        background: {
          light: '#f0f1f5',
          DEFAULT: '#ffffff',
          secondary: '#f8f9fa',
          dark: '#0f172a',
        },
        accent: {
          cyan: '#19c3e6',
          blue: '#2c8aec',
          purple: '#586ce9',
        },
      },
      linearGradient: {
        'btn-gradient': ['90deg', '#20afe7 0%', '#2c8aec 50%', '#586ce9 100%'],
      },
    },
=======
    extend: {},
>>>>>>> b932655445289cc6885ffad4b922c05b464845b2
  },
  plugins: [],
};
