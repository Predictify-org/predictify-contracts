import React from 'react';
import {
  View,
  Text,
  TouchableOpacity,
  StyleSheet,
  Animated,
} from 'react-native';
import { useVoiceRecording } from '../../hooks/useVoiceRecording';

export interface VoiceRecorderProps {
  onRecordingComplete: (uri: string, duration: number) => void;
  onCancel?: () => void;
  maxDuration?: number; // in seconds
}

export const VoiceRecorder: React.FC<VoiceRecorderProps> = ({
  onRecordingComplete,
  onCancel,
  maxDuration = 60,
}) => {
  const {
    startRecording,
    stopRecording,
    cancelRecording,
    isRecording,
    duration,
    error,
  } = useVoiceRecording();

  const [pulseAnim] = React.useState(new Animated.Value(1));

  React.useEffect(() => {
    if (isRecording) {
      Animated.loop(
        Animated.sequence([
          Animated.timing(pulseAnim, {
            toValue: 1.2,
            duration: 500,
            useNativeDriver: true,
          }),
          Animated.timing(pulseAnim, {
            toValue: 1,
            duration: 500,
            useNativeDriver: true,
          }),
        ])
      ).start();
    } else {
      pulseAnim.setValue(1);
    }
  }, [isRecording, pulseAnim]);

  React.useEffect(() => {
    if (duration >= maxDuration && isRecording) {
      handleStop();
    }
  }, [duration, maxDuration, isRecording]);

  const handleStart = async () => {
    const started = await startRecording();
    if (!started && error) {
      console.error('Failed to start recording:', error);
    }
  };

  const handleStop = async () => {
    const result = await stopRecording();
    if (result) {
      onRecordingComplete(result.uri, result.duration);
    }
  };

  const handleCancel = async () => {
    await cancelRecording();
    onCancel?.();
  };

  const formatDuration = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  return (
    <View style={styles.container}>
      <View style={styles.content}>
        {isRecording ? (
          <>
            <Animated.View
              style={[
                styles.recordingIndicator,
                { transform: [{ scale: pulseAnim }] },
              ]}
            >
              <View style={styles.recordingDot} />
            </Animated.View>
            <Text style={styles.duration}>{formatDuration(duration)}</Text>
            <View style={styles.buttons}>
              <TouchableOpacity
                onPress={handleStop}
                style={[styles.button, styles.stopButton]}
              >
                <Text style={styles.stopButtonText}>Stop</Text>
              </TouchableOpacity>
              <TouchableOpacity
                onPress={handleCancel}
                style={[styles.button, styles.cancelButton]}
              >
                <Text style={styles.cancelButtonText}>Cancel</Text>
              </TouchableOpacity>
            </View>
          </>
        ) : (
          <>
            <TouchableOpacity
              onPress={handleStart}
              style={[styles.button, styles.recordButton]}
            >
              <Text style={styles.recordButtonText}>🎤 Start Recording</Text>
            </TouchableOpacity>
            {error && <Text style={styles.error}>{error}</Text>}
          </>
        )}
      </View>
    </View>
  );
};

const styles = StyleSheet.create({
  container: {
    padding: 16,
    backgroundColor: '#fff',
    borderRadius: 12,
    alignItems: 'center',
  },
  content: {
    alignItems: 'center',
    width: '100%',
  },
  recordingIndicator: {
    width: 80,
    height: 80,
    borderRadius: 40,
    backgroundColor: '#ff4444',
    justifyContent: 'center',
    alignItems: 'center',
    marginBottom: 16,
  },
  recordingDot: {
    width: 20,
    height: 20,
    borderRadius: 10,
    backgroundColor: '#fff',
  },
  duration: {
    fontSize: 24,
    fontWeight: '600',
    color: '#333',
    marginBottom: 16,
  },
  buttons: {
    flexDirection: 'row',
    gap: 12,
  },
  button: {
    paddingHorizontal: 24,
    paddingVertical: 12,
    borderRadius: 8,
    minWidth: 100,
    alignItems: 'center',
  },
  recordButton: {
    backgroundColor: '#007AFF',
  },
  recordButtonText: {
    color: '#fff',
    fontSize: 16,
    fontWeight: '600',
  },
  stopButton: {
    backgroundColor: '#ff4444',
  },
  stopButtonText: {
    color: '#fff',
    fontSize: 16,
    fontWeight: '600',
  },
  cancelButton: {
    backgroundColor: '#e0e0e0',
  },
  cancelButtonText: {
    color: '#333',
    fontSize: 16,
    fontWeight: '600',
  },
  error: {
    color: '#ff4444',
    fontSize: 14,
    marginTop: 8,
    textAlign: 'center',
  },
});
