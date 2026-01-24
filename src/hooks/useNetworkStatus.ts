import { useState, useEffect } from 'react';
import * as Network from 'expo-network';
import logger from '../utils/logger';

// Network connection types
export type ConnectionType = 'wifi' | 'cellular' | 'none' | 'unknown';

// Network status interface
export interface NetworkStatus {
  isConnected: boolean;
  isInternetReachable: boolean;
  type: ConnectionType;
}

/**
 * Hook to monitor network connectivity status
 */
export function useNetworkStatus() {
  const [networkStatus, setNetworkStatus] = useState<NetworkStatus>({
    isConnected: false,
    isInternetReachable: false,
    type: 'unknown',
  });

  const [isChecking, setIsChecking] = useState(false);

  /**
   * Check current network status
   */
  const checkNetworkStatus = async (): Promise<NetworkStatus> => {
    try {
      setIsChecking(true);
      
      // Get network state
      const networkState = await Network.getNetworkStateAsync();
      
      const status: NetworkStatus = {
        isConnected: networkState.isConnected ?? false,
        isInternetReachable: networkState.isInternetReachable ?? false,
        type: 'unknown', // expo-network doesn't provide type info
      };

      // Note: expo-network only provides basic connectivity info
      // Advanced network type detection would require additional native modules

      setNetworkStatus(status);
      return status;
    } catch (error) {
      logger.error('Error checking network status:', error);
      
      // Return offline status on error
      const errorStatus: NetworkStatus = {
        isConnected: false,
        isInternetReachable: false,
        type: 'unknown',
      };
      
      setNetworkStatus(errorStatus);
      return errorStatus;
    } finally {
      setIsChecking(false);
    }
  };

  // Note: expo-network doesn't expose network type enums
  // We'll use 'unknown' for now

  /**
   * Check if device is online (connected with internet access)
   */
  const isOnline = (): boolean => {
    return networkStatus.isConnected && networkStatus.isInternetReachable;
  };

  /**
   * Check if device is offline
   */
  const isOffline = (): boolean => {
    return !networkStatus.isConnected || !networkStatus.isInternetReachable;
  };

  /**
   * Check if on fast connection (assuming WiFi is fast)
   */
  const isFastConnection = (): boolean => {
    // Since we can't detect network type, we'll assume connected is reasonably fast
    // In a real app, you'd want more sophisticated detection
    return networkStatus.isConnected && networkStatus.isInternetReachable;
  };

  /**
   * Monitor network changes
   */
  useEffect(() => {
    let isMounted = true;

    const monitorNetwork = async () => {
      if (!isMounted) return;
      
      // Initial check
      await checkNetworkStatus();

      // Set up interval for periodic checks
      const intervalId = setInterval(async () => {
        if (isMounted) {
          await checkNetworkStatus();
        }
      }, 5000); // Check every 5 seconds

      return () => {
        isMounted = false;
        clearInterval(intervalId);
      };
    };

    const cleanup = monitorNetwork();
    
    return () => {
      cleanup.then(unsubscribe => unsubscribe && unsubscribe());
    };
  }, []);

  return {
    // Current network status
    networkStatus,
    
    // Status checkers
    isOnline: isOnline(),
    isOffline: isOffline(),
    isFastConnection: isFastConnection(),
    
    // Loading state
    isChecking,
    
    // Manual refresh
    refresh: checkNetworkStatus,
  };
}

/**
 * Hook specifically for monitoring online/offline status
 */
export function useOnlineStatus() {
  const { isOnline, isOffline, refresh } = useNetworkStatus();
  
  return {
    isOnline,
    isOffline,
    refresh,
  };
}

/**
 * Hook for connection quality monitoring
 */
export function useConnectionQuality() {
  const { networkStatus, isFastConnection, refresh } = useNetworkStatus();
  
  return {
    connectionType: networkStatus.type,
    isFastConnection,
    refresh,
  };
}

export default useNetworkStatus;