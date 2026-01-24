/**
 * Example usage of MobileChat component
 * 
 * This file demonstrates how to integrate the MobileChat component
 * into your app screens or navigation.
 */

import React from 'react';
import { View } from 'react-native';
import { MobileChat } from './MobileChat';

export function ChatScreenExample() {
  return (
    <View style={{ flex: 1 }}>
      <MobileChat
        chatId="chat_123"
        currentUserId="user_456"
        currentUserName="John Doe"
        currentUserAvatar="https://example.com/avatar.jpg"
        onSendMessage={(message) => {
          // Optional: Handle message sending
          console.log('Message sent:', message);
        }}
        initialMessages={[
          {
            id: 'msg_1',
            chatId: 'chat_123',
            userId: 'user_789',
            userName: 'Jane Smith',
            type: 'text',
            content: 'Hello! How are you?',
            timestamp: Date.now() - 3600000,
            reactions: [],
          },
        ]}
      />
    </View>
  );
}

/**
 * To use in navigation:
 * 
 * 1. Add to navigation types (src/navigation/types.ts):
 * 
 * export type RootStackParamList = {
 *   Home: undefined;
 *   Profile: { userId: string };
 *   Settings: undefined;
 *   Chat: { chatId: string }; // Add this
 * };
 * 
 * 2. Create ChatScreen.tsx:
 * 
 * import React from 'react';
 * import { MobileChat } from '../components/mobile/MobileChat';
 * import { NativeStackScreenProps } from '@react-navigation/native-stack';
 * import { RootStackParamList } from '../navigation/types';
 * 
 * type Props = NativeStackScreenProps<RootStackParamList, 'Chat'>;
 * 
 * export default function ChatScreen({ route }: Props) {
 *   const { chatId } = route.params;
 *   
 *   return (
 *     <MobileChat
 *       chatId={chatId}
 *       currentUserId="current_user_id"
 *       currentUserName="Current User"
 *     />
 *   );
 * }
 * 
 * 3. Add to navigator:
 * 
 * <Stack.Screen name="Chat" component={ChatScreen} />
 */
