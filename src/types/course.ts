export interface Lesson {
  id: string;
  title: string;
  content: string;
  duration: number; // in minutes
  videoUrl?: string;
  resources?: Resource[];
  order: number;
}

export interface Resource {
  id: string;
  title: string;
  url: string;
  type: 'pdf' | 'link' | 'code' | 'image';
}

export interface Section {
  id: string;
  title: string;
  lessons: Lesson[];
  order: number;
}

export interface Course {
  id: string;
  title: string;
  description: string;
  instructor: {
    id: string;
    name: string;
    avatar?: string;
  };
  thumbnail?: string;
  sections: Section[];
  totalLessons: number;
  totalDuration: number; // in minutes
  level: 'beginner' | 'intermediate' | 'advanced';
  category: string;
}

export interface LessonProgress {
  lessonId: string;
  completed: boolean;
  lastPosition: number; // timestamp in seconds for video/audio, or scroll position
  completedAt?: string;
  timeSpent: number; // in seconds
}

export interface CourseProgress {
  courseId: string;
  currentLessonId: string;
  currentSectionId: string;
  lessons: Record<string, LessonProgress>;
  overallProgress: number; // 0-100
  lastAccessed: string;
  bookmarks: string[]; // lesson IDs
  notes: Record<string, Note[]>; // lessonId -> notes
}

export interface Note {
  id: string;
  lessonId: string;
  content: string;
  timestamp: number; // position in lesson
  createdAt: string;
  updatedAt: string;
}

export interface Bookmark {
  id: string;
  courseId: string;
  lessonId: string;
  createdAt: string;
  note?: string;
}
