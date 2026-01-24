export type MessageType = 'text' | 'image' | 'file' | 'voice';

export type ReactionType = 'like' | 'love' | 'laugh' | 'wow' | 'sad' | 'angry';

export interface MessageReaction {
  type: ReactionType;
  userId: string;
  userName?: string;
}

export interface Message {
  id: string;
  chatId: string;
  userId: string;
  userName: string;
  userAvatar?: string;
  type: MessageType;
  content: string; // text content or URI for media
  timestamp: number;
  reactions?: MessageReaction[];
  isRead?: boolean;
  isEdited?: boolean;
  replyTo?: string; // message ID this is replying to
}

export interface TypingIndicator {
  userId: string;
  userName: string;
  chatId: string;
}

export interface ChatAttachment {
  uri: string;
  type: 'image' | 'file';
  name?: string;
  size?: number;
}
