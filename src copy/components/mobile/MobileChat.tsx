import React, { useState, useEffect, useRef, useCallback } from 'react';
import {
  View,
  Text,
  TextInput,
  TouchableOpacity,
  FlatList,
  Image,
  StyleSheet,
  KeyboardAvoidingView,
  Platform,
  ActivityIndicator,
  Modal,
} from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';
import SocketService from '../../services/socket';
import { ChatBubble } from './ChatBubble';
import { VoiceRecorder } from './VoiceRecorder';
import { useImagePicker } from '../../hooks/useImagePicker';
import type { Message, TypingIndicator, ReactionType } from '../../types/message';

export interface MobileChatProps {
  chatId: string;
  currentUserId: string;
  currentUserName: string;
  currentUserAvatar?: string;
  onSendMessage?: (message: Omit<Message, 'id' | 'timestamp'>) => void;
  initialMessages?: Message[];
}

export const MobileChat: React.FC<MobileChatProps> = ({
  chatId,
  currentUserId,
  currentUserName,
  currentUserAvatar,
  onSendMessage,
  initialMessages = [],
}) => {
  const [messages, setMessages] = useState<Message[]>(initialMessages);
  const [inputText, setInputText] = useState('');
  const [typingUsers, setTypingUsers] = useState<TypingIndicator[]>([]);
  const [showVoiceRecorder, setShowVoiceRecorder] = useState(false);
  const [showReactionPicker, setShowReactionPicker] = useState(false);
  const [selectedMessage, setSelectedMessage] = useState<Message | null>(null);
  const flatListRef = useRef<FlatList>(null);
  const typingTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  const { pickImage, takePhoto, picking } = useImagePicker();

  // Socket event handlers
  useEffect(() => {
    const socket = SocketService.connect();

    // Listen for new messages
    socket.on('message', (message: Message) => {
      if (message.chatId === chatId) {
        setMessages((prev) => {
          // Avoid duplicates
          if (prev.some((m) => m.id === message.id)) return prev;
          return [...prev, message];
        });
        scrollToBottom();
      }
    });

    // Listen for typing indicators
    socket.on('typing', (indicator: TypingIndicator) => {
      if (indicator.chatId === chatId && indicator.userId !== currentUserId) {
        setTypingUsers((prev) => {
          const existing = prev.find((u) => u.userId === indicator.userId);
          if (existing) return prev;
          return [...prev, indicator];
        });

        // Remove typing indicator after 3 seconds
        setTimeout(() => {
          setTypingUsers((prev) =>
            prev.filter((u) => u.userId !== indicator.userId)
          );
        }, 3000);
      }
    });

    // Listen for message reactions
    socket.on('messageReaction', (data: { messageId: string; reaction: ReactionType; userId: string }) => {
      setMessages((prev) =>
        prev.map((msg) => {
          if (msg.id === data.messageId) {
            const reactions = msg.reactions || [];
            const existingIndex = reactions.findIndex(
              (r) => r.userId === data.userId
            );
            const newReactions = [...reactions];
            if (existingIndex >= 0) {
              newReactions[existingIndex] = {
                type: data.reaction,
                userId: data.userId,
              };
            } else {
              newReactions.push({
                type: data.reaction,
                userId: data.userId,
              });
            }
            return { ...msg, reactions: newReactions };
          }
          return msg;
        })
      );
    });

    return () => {
      socket.off('message');
      socket.off('typing');
      socket.off('messageReaction');
    };
  }, [chatId, currentUserId]);

  const scrollToBottom = useCallback(() => {
    setTimeout(() => {
      flatListRef.current?.scrollToEnd({ animated: true });
    }, 100);
  }, []);

  const sendTypingIndicator = useCallback(() => {
    if (typingTimeoutRef.current) {
      clearTimeout(typingTimeoutRef.current);
    }

    SocketService.emit('typing', {
      chatId,
      userId: currentUserId,
      userName: currentUserName,
    });

    typingTimeoutRef.current = setTimeout(() => {
      // Stop typing indicator after 2 seconds of no input
    }, 2000);
  }, [chatId, currentUserId, currentUserName]);

  const handleSendMessage = useCallback(
    (content: string, type: Message['type'] = 'text') => {
      if (!content.trim() && type === 'text') return;

      const newMessage: Message = {
        id: `msg_${Date.now()}_${Math.random()}`,
        chatId,
        userId: currentUserId,
        userName: currentUserName,
        userAvatar: currentUserAvatar,
        type,
        content: content.trim(),
        timestamp: Date.now(),
        reactions: [],
        isRead: false,
      };

      setMessages((prev) => [...prev, newMessage]);
      setInputText('');
      scrollToBottom();

      // Emit to socket
      SocketService.emit('message', newMessage);

      // Call optional callback
      onSendMessage?.(newMessage);

      // Simulate acknowledgment (in real app, wait for server confirmation)
      setTimeout(() => {
        setMessages((prev) =>
          prev.map((msg) =>
            msg.id === newMessage.id ? { ...msg, isRead: true } : msg
          )
        );
      }, 500);
    },
    [chatId, currentUserId, currentUserName, currentUserAvatar, onSendMessage, scrollToBottom]
  );

  const handleImagePick = useCallback(async () => {
    const result = await pickImage();
    if (result) {
      handleSendMessage(result.uri, 'image');
    }
  }, [pickImage, handleSendMessage]);

  const handleTakePhoto = useCallback(async () => {
    const result = await takePhoto();
    if (result) {
      handleSendMessage(result.uri, 'image');
    }
  }, [takePhoto, handleSendMessage]);

  const handleVoiceComplete = useCallback(
    (uri: string, duration: number) => {
      handleSendMessage(uri, 'voice');
      setShowVoiceRecorder(false);
    },
    [handleSendMessage]
  );

  const handleReaction = useCallback(
    (messageId: string, reaction: ReactionType) => {
      SocketService.emit('messageReaction', {
        messageId,
        reaction,
        userId: currentUserId,
      });

      // Optimistically update UI
      setMessages((prev) =>
        prev.map((msg) => {
          if (msg.id === messageId) {
            const reactions = msg.reactions || [];
            const existingIndex = reactions.findIndex(
              (r) => r.userId === currentUserId
            );
            const newReactions = [...reactions];
            if (existingIndex >= 0) {
              newReactions[existingIndex] = {
                type: reaction,
                userId: currentUserId,
              };
            } else {
              newReactions.push({
                type: reaction,
                userId: currentUserId,
              });
            }
            return { ...msg, reactions: newReactions };
          }
          return msg;
        })
      );
      setShowReactionPicker(false);
      setSelectedMessage(null);
    },
    [currentUserId]
  );

  const handleLongPress = useCallback((message: Message) => {
    setSelectedMessage(message);
    setShowReactionPicker(true);
  }, []);

  const renderMessage = useCallback(
    ({ item }: { item: Message }) => (
      <ChatBubble
        message={item}
        isOwn={item.userId === currentUserId}
        onReaction={handleReaction}
        onLongPress={handleLongPress}
      />
    ),
    [currentUserId, handleReaction, handleLongPress]
  );

  const renderTypingIndicator = () => {
    if (typingUsers.length === 0) return null;
    return (
      <View style={styles.typingContainer}>
        <Text style={styles.typingText}>
          {typingUsers.map((u) => u.userName).join(', ')} typing...
        </Text>
        <ActivityIndicator size="small" color="#999" />
      </View>
    );
  };

  return (
    <SafeAreaView style={styles.container} edges={['top', 'bottom']}>
      <KeyboardAvoidingView
        style={styles.keyboardView}
        behavior={Platform.OS === 'ios' ? 'padding' : undefined}
        keyboardVerticalOffset={Platform.OS === 'ios' ? 90 : 0}
      >
        {/* Messages List */}
        <FlatList
          ref={flatListRef}
          data={messages}
          renderItem={renderMessage}
          keyExtractor={(item) => item.id}
          contentContainerStyle={styles.messagesList}
          inverted={false}
          onContentSizeChange={scrollToBottom}
          ListFooterComponent={renderTypingIndicator}
        />

        {/* Input Area */}
        <View style={styles.inputContainer}>
          <TouchableOpacity
            onPress={() => setShowVoiceRecorder(true)}
            style={styles.attachButton}
          >
            <Text style={styles.attachButtonText}>🎤</Text>
          </TouchableOpacity>

          <TouchableOpacity
            onPress={handleImagePick}
            style={styles.attachButton}
            disabled={picking}
          >
            {picking ? (
              <ActivityIndicator size="small" color="#007AFF" />
            ) : (
              <Text style={styles.attachButtonText}>📷</Text>
            )}
          </TouchableOpacity>

          <TextInput
            style={styles.input}
            value={inputText}
            onChangeText={(text) => {
              setInputText(text);
              sendTypingIndicator();
            }}
            placeholder="Type a message..."
            multiline
            maxLength={1000}
          />

          <TouchableOpacity
            onPress={() => handleSendMessage(inputText)}
            style={[
              styles.sendButton,
              !inputText.trim() && styles.sendButtonDisabled,
            ]}
            disabled={!inputText.trim()}
          >
            <Text style={styles.sendButtonText}>Send</Text>
          </TouchableOpacity>
        </View>

        {/* Voice Recorder Modal */}
        <Modal
          visible={showVoiceRecorder}
          transparent
          animationType="slide"
          onRequestClose={() => setShowVoiceRecorder(false)}
        >
          <View style={styles.modalOverlay}>
            <View style={styles.modalContent}>
              <TouchableOpacity
                style={styles.modalCloseButton}
                onPress={() => setShowVoiceRecorder(false)}
              >
                <Text style={styles.modalCloseText}>✕</Text>
              </TouchableOpacity>
              <VoiceRecorder
                onRecordingComplete={handleVoiceComplete}
                onCancel={() => setShowVoiceRecorder(false)}
              />
            </View>
          </View>
        </Modal>

        {/* Reaction Picker Modal */}
        <Modal
          visible={showReactionPicker}
          transparent
          animationType="fade"
          onRequestClose={() => setShowReactionPicker(false)}
        >
          <TouchableOpacity
            style={styles.modalOverlay}
            activeOpacity={1}
            onPress={() => setShowReactionPicker(false)}
          >
            <View style={styles.reactionPicker}>
              {(['like', 'love', 'laugh', 'wow', 'sad', 'angry'] as ReactionType[]).map(
                (reaction) => (
                  <TouchableOpacity
                    key={reaction}
                    onPress={() =>
                      selectedMessage &&
                      handleReaction(selectedMessage.id, reaction)
                    }
                    style={styles.reactionOption}
                  >
                    <Text style={styles.reactionOptionEmoji}>
                      {reaction === 'like' && '👍'}
                      {reaction === 'love' && '❤️'}
                      {reaction === 'laugh' && '😂'}
                      {reaction === 'wow' && '😮'}
                      {reaction === 'sad' && '😢'}
                      {reaction === 'angry' && '😠'}
                    </Text>
                  </TouchableOpacity>
                )
              )}
            </View>
          </TouchableOpacity>
        </Modal>
      </KeyboardAvoidingView>
    </SafeAreaView>
  );
};

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#fff',
  },
  keyboardView: {
    flex: 1,
  },
  messagesList: {
    paddingVertical: 8,
  },
  inputContainer: {
    flexDirection: 'row',
    alignItems: 'flex-end',
    paddingHorizontal: 12,
    paddingVertical: 8,
    borderTopWidth: 1,
    borderTopColor: '#e0e0e0',
    backgroundColor: '#fff',
  },
  attachButton: {
    padding: 8,
    marginRight: 8,
    justifyContent: 'center',
    alignItems: 'center',
  },
  attachButtonText: {
    fontSize: 20,
  },
  input: {
    flex: 1,
    borderWidth: 1,
    borderColor: '#e0e0e0',
    borderRadius: 20,
    paddingHorizontal: 16,
    paddingVertical: 8,
    maxHeight: 100,
    fontSize: 16,
  },
  sendButton: {
    paddingHorizontal: 16,
    paddingVertical: 8,
    backgroundColor: '#007AFF',
    borderRadius: 20,
    marginLeft: 8,
    justifyContent: 'center',
  },
  sendButtonDisabled: {
    backgroundColor: '#ccc',
  },
  sendButtonText: {
    color: '#fff',
    fontWeight: '600',
    fontSize: 16,
  },
  typingContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: 12,
    paddingVertical: 4,
  },
  typingText: {
    fontSize: 12,
    color: '#999',
    marginRight: 8,
    fontStyle: 'italic',
  },
  modalOverlay: {
    flex: 1,
    backgroundColor: 'rgba(0, 0, 0, 0.5)',
    justifyContent: 'center',
    alignItems: 'center',
  },
  modalContent: {
    backgroundColor: '#fff',
    borderRadius: 16,
    padding: 24,
    width: '80%',
    maxWidth: 400,
  },
  modalCloseButton: {
    position: 'absolute',
    top: 8,
    right: 8,
    padding: 8,
    zIndex: 1,
  },
  modalCloseText: {
    fontSize: 24,
    color: '#999',
  },
  reactionPicker: {
    flexDirection: 'row',
    backgroundColor: '#fff',
    borderRadius: 24,
    padding: 8,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.25,
    shadowRadius: 4,
    elevation: 5,
  },
  reactionOption: {
    padding: 8,
    marginHorizontal: 4,
  },
  reactionOptionEmoji: {
    fontSize: 28,
  },
});
