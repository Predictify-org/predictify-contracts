import React, { useState } from 'react';
import {
  View,
  Text,
  Image,
  TouchableOpacity,
  StyleSheet,
  Pressable,
  ActivityIndicator,
} from 'react-native';
import { Audio } from 'expo-av';
import type { Message, ReactionType } from '../../types/message';

export interface ChatBubbleProps {
  message: Message;
  isOwn: boolean;
  onReaction?: (messageId: string, reaction: ReactionType) => void;
  onLongPress?: (message: Message) => void;
  onPress?: (message: Message) => void;
}

const REACTION_EMOJIS: Record<ReactionType, string> = {
  like: '👍',
  love: '❤️',
  laugh: '😂',
  wow: '😮',
  sad: '😢',
  angry: '😠',
};

export const ChatBubble: React.FC<ChatBubbleProps> = ({
  message,
  isOwn,
  onReaction,
  onLongPress,
  onPress,
}) => {
  const [sound, setSound] = useState<Audio.Sound | null>(null);
  const [playing, setPlaying] = useState(false);
  const [imageError, setImageError] = useState(false);

  const formatTime = (timestamp: number) => {
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffMins < 1440) return `${Math.floor(diffMins / 60)}h ago`;

    return date.toLocaleDateString();
  };

  const playVoiceMessage = async () => {
    if (message.type !== 'voice' || !message.content) return;

    try {
      if (sound) {
        await sound.unloadAsync();
      }

      const { sound: newSound } = await Audio.Sound.createAsync(
        { uri: message.content },
        { shouldPlay: true }
      );

      setSound(newSound);
      setPlaying(true);

      newSound.setOnPlaybackStatusUpdate((status) => {
        if (status.isLoaded && status.didJustFinish) {
          setPlaying(false);
        }
      });
    } catch (err) {
      console.error('Failed to play voice message:', err);
    }
  };

  const stopVoiceMessage = async () => {
    if (sound) {
      await sound.unloadAsync();
      setSound(null);
      setPlaying(false);
    }
  };

  React.useEffect(() => {
    return () => {
      if (sound) {
        sound.unloadAsync();
      }
    };
  }, [sound]);

  const renderContent = () => {
    switch (message.type) {
      case 'image':
        if (imageError) {
          return (
            <View style={styles.imageErrorContainer}>
              <Text style={styles.imageErrorText}>Failed to load image</Text>
            </View>
          );
        }
        return (
          <Image
            source={{ uri: message.content }}
            style={styles.messageImage}
            onError={() => setImageError(true)}
            resizeMode="cover"
          />
        );

      case 'voice':
        return (
          <TouchableOpacity
            onPress={playing ? stopVoiceMessage : playVoiceMessage}
            style={styles.voiceContainer}
          >
            {playing ? (
              <ActivityIndicator size="small" color={isOwn ? '#fff' : '#007AFF'} />
            ) : (
              <Text style={[styles.voiceIcon, isOwn && styles.voiceIconOwn]}>
                🎤
              </Text>
            )}
            <Text style={[styles.voiceText, isOwn && styles.voiceTextOwn]}>
              {playing ? 'Playing...' : 'Tap to play'}
            </Text>
          </TouchableOpacity>
        );

      case 'file':
        return (
          <View style={styles.fileContainer}>
            <Text style={styles.fileIcon}>📎</Text>
            <Text style={[styles.fileText, isOwn && styles.fileTextOwn]}>
              {message.content.split('/').pop() || 'File'}
            </Text>
          </View>
        );

      default:
        return (
          <Text style={[styles.messageText, isOwn && styles.messageTextOwn]}>
            {message.content}
          </Text>
        );
    }
  };

  const renderReactions = () => {
    if (!message.reactions || message.reactions.length === 0) return null;

    const reactionGroups: Record<ReactionType, number> = {
      like: 0,
      love: 0,
      laugh: 0,
      wow: 0,
      sad: 0,
      angry: 0,
    };

    message.reactions.forEach((r) => {
      reactionGroups[r.type] = (reactionGroups[r.type] || 0) + 1;
    });

    return (
      <View style={styles.reactionsContainer}>
        {Object.entries(reactionGroups).map(([type, count]) => {
          if (count === 0) return null;
          return (
            <View key={type} style={styles.reactionBadge}>
              <Text style={styles.reactionEmoji}>
                {REACTION_EMOJIS[type as ReactionType]}
              </Text>
              {count > 1 && (
                <Text style={styles.reactionCount}>{count}</Text>
              )}
            </View>
          );
        })}
      </View>
    );
  };

  return (
    <Pressable
      onPress={() => onPress?.(message)}
      onLongPress={() => onLongPress?.(message)}
      style={[styles.container, isOwn && styles.containerOwn]}
    >
      {!isOwn && message.userAvatar && (
        <Image source={{ uri: message.userAvatar }} style={styles.avatar} />
      )}

      <View style={[styles.bubble, isOwn && styles.bubbleOwn]}>
        {!isOwn && (
          <Text style={styles.userName}>{message.userName}</Text>
        )}

        {renderContent()}

        <View style={styles.footer}>
          <Text style={[styles.timestamp, isOwn && styles.timestampOwn]}>
            {formatTime(message.timestamp)}
          </Text>
          {message.isEdited && (
            <Text style={[styles.edited, isOwn && styles.editedOwn]}>
              (edited)
            </Text>
          )}
        </View>

        {renderReactions()}
      </View>

      {onReaction && (
        <View style={styles.reactionButtons}>
          {Object.entries(REACTION_EMOJIS).map(([type, emoji]) => (
            <TouchableOpacity
              key={type}
              onPress={() => onReaction(message.id, type as ReactionType)}
              style={styles.reactionButton}
            >
              <Text style={styles.reactionButtonEmoji}>{emoji}</Text>
            </TouchableOpacity>
          ))}
        </View>
      )}
    </Pressable>
  );
};

const styles = StyleSheet.create({
  container: {
    flexDirection: 'row',
    marginVertical: 4,
    paddingHorizontal: 12,
    alignItems: 'flex-end',
  },
  containerOwn: {
    flexDirection: 'row-reverse',
  },
  avatar: {
    width: 32,
    height: 32,
    borderRadius: 16,
    marginRight: 8,
    marginBottom: 4,
  },
  bubble: {
    maxWidth: '75%',
    backgroundColor: '#f1f1f1',
    borderRadius: 18,
    paddingHorizontal: 12,
    paddingVertical: 8,
    borderBottomLeftRadius: 4,
  },
  bubbleOwn: {
    backgroundColor: '#007AFF',
    borderBottomLeftRadius: 18,
    borderBottomRightRadius: 4,
  },
  userName: {
    fontSize: 12,
    fontWeight: '600',
    color: '#666',
    marginBottom: 4,
  },
  messageText: {
    fontSize: 16,
    color: '#000',
    lineHeight: 20,
  },
  messageTextOwn: {
    color: '#fff',
  },
  messageImage: {
    width: 200,
    height: 200,
    borderRadius: 12,
    marginVertical: 4,
  },
  imageErrorContainer: {
    width: 200,
    height: 200,
    backgroundColor: '#e0e0e0',
    borderRadius: 12,
    justifyContent: 'center',
    alignItems: 'center',
  },
  imageErrorText: {
    color: '#999',
    fontSize: 14,
  },
  voiceContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingVertical: 8,
  },
  voiceIcon: {
    fontSize: 20,
    marginRight: 8,
  },
  voiceIconOwn: {
    filter: 'brightness(0) invert(1)',
  },
  voiceText: {
    fontSize: 14,
    color: '#000',
  },
  voiceTextOwn: {
    color: '#fff',
  },
  fileContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingVertical: 4,
  },
  fileIcon: {
    fontSize: 20,
    marginRight: 8,
  },
  fileText: {
    fontSize: 14,
    color: '#007AFF',
    textDecorationLine: 'underline',
  },
  fileTextOwn: {
    color: '#fff',
  },
  footer: {
    flexDirection: 'row',
    alignItems: 'center',
    marginTop: 4,
  },
  timestamp: {
    fontSize: 11,
    color: '#999',
  },
  timestampOwn: {
    color: 'rgba(255, 255, 255, 0.7)',
  },
  edited: {
    fontSize: 11,
    color: '#999',
    marginLeft: 4,
    fontStyle: 'italic',
  },
  editedOwn: {
    color: 'rgba(255, 255, 255, 0.7)',
  },
  reactionsContainer: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    marginTop: 4,
    gap: 4,
  },
  reactionBadge: {
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: 'rgba(0, 0, 0, 0.1)',
    borderRadius: 12,
    paddingHorizontal: 6,
    paddingVertical: 2,
  },
  reactionEmoji: {
    fontSize: 14,
  },
  reactionCount: {
    fontSize: 11,
    marginLeft: 2,
    color: '#666',
  },
  reactionButtons: {
    flexDirection: 'row',
    marginLeft: 8,
    gap: 4,
  },
  reactionButton: {
    padding: 4,
  },
  reactionButtonEmoji: {
    fontSize: 18,
  },
});
