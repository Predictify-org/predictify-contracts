import axios from "axios";

const apiClient = axios.create({
  baseURL: process.env.EXPO_PUBLIC_API_BASE_URL || "http://localhost:3000",
  timeout: 10000,
  headers: {
    "Content-Type": "application/json",
  },
});

// Request interceptor
apiClient.interceptors.request.use(
  (config) => {
    // Add auth token here when available
    // const token = getAuthToken();
    // if (token) config.headers.Authorization = `Bearer ${token}`;
    return config;
  },
  (error) => Promise.reject(error),
);

// Response interceptor
apiClient.interceptors.response.use(
  (response) => response,
  (error) => {
    // Only log API errors in development, and make network errors less noisy
    if (__DEV__) {
      if (error.code === 'ERR_NETWORK' || error.message === 'Network Error') {
        // Network errors are expected when backend isn't running - just warn
        console.warn("⚠️ API not available (running in offline mode)");
      } else {
        console.error("API Error:", error.response?.data || error.message);
      }
    }
    return Promise.reject(error);
  },
);

export default apiClient;
