import { useState, useRef, useEffect } from 'react';
import { Audio } from 'expo-av';

export interface VoiceRecordingResult {
  uri: string;
  duration: number;
}

export const useVoiceRecording = () => {
  const [recording, setRecording] = useState<Audio.Recording | null>(null);
  const [isRecording, setIsRecording] = useState(false);
  const [duration, setDuration] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [hasPermission, setHasPermission] = useState<boolean | null>(null);
  const durationIntervalRef = useRef<NodeJS.Timeout | null>(null);
  const startTimeRef = useRef<number | null>(null);

  useEffect(() => {
    requestPermissions();
    return () => {
      stopRecording();
      if (durationIntervalRef.current) {
        clearInterval(durationIntervalRef.current);
      }
    };
  }, []);

  const requestPermissions = async () => {
    try {
      const { status } = await Audio.requestPermissionsAsync();
      setHasPermission(status === 'granted');
    } catch (err) {
      setError('Failed to request audio permissions');
      setHasPermission(false);
    }
  };

  const startRecording = async (): Promise<boolean> => {
    try {
      if (!hasPermission) {
        await requestPermissions();
        if (!hasPermission) {
          setError('Audio recording permission is required');
          return false;
        }
      }

      setError(null);
      await Audio.setAudioModeAsync({
        allowsRecordingIOS: true,
        playsInSilentModeIOS: true,
      });

      const { recording: newRecording } = await Audio.Recording.createAsync(
        Audio.RecordingOptionsPresets.HIGH_QUALITY
      );

      setRecording(newRecording);
      setIsRecording(true);
      startTimeRef.current = Date.now();
      setDuration(0);

      // Update duration every 100ms
      durationIntervalRef.current = setInterval(() => {
        if (startTimeRef.current) {
          setDuration(Math.floor((Date.now() - startTimeRef.current) / 1000));
        }
      }, 100);

      return true;
    } catch (err) {
      const message =
        err instanceof Error ? err.message : 'Failed to start recording';
      setError(message);
      setIsRecording(false);
      return false;
    }
  };

  const stopRecording = async (): Promise<VoiceRecordingResult | null> => {
    try {
      if (!recording) return null;

      setIsRecording(false);
      if (durationIntervalRef.current) {
        clearInterval(durationIntervalRef.current);
        durationIntervalRef.current = null;
      }

      await recording.stopAndUnloadAsync();
      const uri = recording.getURI();
      const finalDuration = duration;

      setRecording(null);
      setDuration(0);
      startTimeRef.current = null;

      if (!uri) {
        setError('Failed to get recording URI');
        return null;
      }

      return {
        uri,
        duration: finalDuration,
      };
    } catch (err) {
      const message =
        err instanceof Error ? err.message : 'Failed to stop recording';
      setError(message);
      return null;
    }
  };

  const cancelRecording = async () => {
    if (recording) {
      try {
        await recording.stopAndUnloadAsync();
      } catch (err) {
        // Ignore errors on cancel
      }
    }
    setRecording(null);
    setIsRecording(false);
    setDuration(0);
    startTimeRef.current = null;
    if (durationIntervalRef.current) {
      clearInterval(durationIntervalRef.current);
      durationIntervalRef.current = null;
    }
  };

  return {
    startRecording,
    stopRecording,
    cancelRecording,
    isRecording,
    duration,
    hasPermission,
    error,
  };
};
