import React, { useEffect } from 'react';
import { StatusBar } from 'expo-status-bar';
<<<<<<< HEAD
import { LogBox } from 'react-native';
import AppNavigator from './src/navigation/AppNavigator';
import { useAppStore } from './src/store';
import socketService from './src/services/socket';
import { ErrorBoundary } from './src/components/common/ErrorBoundary';
import "./global.css";

// Enable error logging to console (visible in Metro bundler)
if (__DEV__) {
  // Log all errors to console
  const originalError = console.error;
  console.error = (...args) => {
    originalError(...args);
    // Errors will appear in Metro bundler terminal
  };

  // Show warnings in console but don't break the app
  LogBox.ignoreLogs([
    'Non-serializable values were found in the navigation state',
  ]);
}

=======
import AppNavigator from './src/navigation/AppNavigator';
import { useAppStore } from './src/store';
import socketService from './src/services/socket';
import "./global.css";

>>>>>>> b932655445289cc6885ffad4b922c05b464845b2
export default function App() {
  const theme = useAppStore((state) => state.theme);

  useEffect(() => {
    // Connect to socket when app starts
    socketService.connect();

    // Cleanup on unmount
    return () => {
      socketService.disconnect();
    };
  }, []);

  return (
<<<<<<< HEAD
    <ErrorBoundary>
      <StatusBar style={theme === 'dark' ? 'light' : 'dark'} />
      <AppNavigator />
    </ErrorBoundary>
=======
    <>
      <StatusBar style={theme === 'dark' ? 'light' : 'dark'} />
      <AppNavigator />
    </>
>>>>>>> b932655445289cc6885ffad4b922c05b464845b2
  );
}