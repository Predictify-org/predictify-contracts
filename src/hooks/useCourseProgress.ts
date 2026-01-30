import { useState, useEffect, useCallback } from 'react';
import AsyncStorage from '@react-native-async-storage/async-storage';
import { CourseProgress, LessonProgress, Note, Course } from '../types/course';
import apiService from '../services/api';
import logger from '../utils/logger';

const PROGRESS_STORAGE_KEY = '@teachlink_course_progress';
const SYNC_INTERVAL = 30000; // 30 seconds

interface UseCourseProgressOptions {
  courseId: string;
  course?: Course;
  autoSync?: boolean;
}

interface UseCourseProgressReturn {
  progress: CourseProgress | null;
  isLoading: boolean;
  updateLessonProgress: (lessonId: string, progress: Partial<LessonProgress>) => Promise<void>;
  markLessonComplete: (lessonId: string) => Promise<void>;
  setCurrentLesson: (lessonId: string, sectionId: string) => Promise<void>;
  addBookmark: (lessonId: string) => Promise<void>;
  removeBookmark: (lessonId: string) => Promise<void>;
  addNote: (lessonId: string, content: string, timestamp: number) => Promise<Note>;
  updateNote: (lessonId: string, noteId: string, content: string) => Promise<void>;
  deleteNote: (lessonId: string, noteId: string) => Promise<void>;
  updateLastPosition: (lessonId: string, position: number) => Promise<void>;
  calculateOverallProgress: () => number;
  syncProgress: () => Promise<void>;
}

export function useCourseProgress({
  courseId,
  course,
  autoSync = true,
}: UseCourseProgressOptions): UseCourseProgressReturn {
  const [progress, setProgress] = useState<CourseProgress | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  // Load progress from AsyncStorage
  const loadProgress = useCallback(async () => {
    try {
      setIsLoading(true);
      const stored = await AsyncStorage.getItem(`${PROGRESS_STORAGE_KEY}_${courseId}`);
      
      if (stored) {
        const parsed = JSON.parse(stored) as CourseProgress;
        setProgress(parsed);
      } else {
        // Initialize new progress
        const initialProgress: CourseProgress = {
          courseId,
          currentLessonId: course?.sections[0]?.lessons[0]?.id || '',
          currentSectionId: course?.sections[0]?.id || '',
          lessons: {},
          overallProgress: 0,
          lastAccessed: new Date().toISOString(),
          bookmarks: [],
          notes: {},
        };
        setProgress(initialProgress);
        await AsyncStorage.setItem(
          `${PROGRESS_STORAGE_KEY}_${courseId}`,
          JSON.stringify(initialProgress)
        );
      }
    } catch (error) {
      logger.error('Error loading progress:', error);
      // Initialize on error
      const initialProgress: CourseProgress = {
        courseId,
        currentLessonId: course?.sections[0]?.lessons[0]?.id || '',
        currentSectionId: course?.sections[0]?.id || '',
        lessons: {},
        overallProgress: 0,
        lastAccessed: new Date().toISOString(),
        bookmarks: [],
        notes: {},
      };
      setProgress(initialProgress);
    } finally {
      setIsLoading(false);
    }
  }, [courseId, course]);

  // Save progress to AsyncStorage
  const saveProgress = useCallback(async (updatedProgress: CourseProgress) => {
    try {
      await AsyncStorage.setItem(
        `${PROGRESS_STORAGE_KEY}_${courseId}`,
        JSON.stringify(updatedProgress)
      );
      setProgress(updatedProgress);
    } catch (error) {
      logger.error('Error saving progress:', error);
    }
  }, [courseId]);

  // Sync progress with server
  const syncProgress = useCallback(async () => {
    if (!progress) return;
    
    // Check if API is available (not localhost or empty)
    const apiUrl = process.env.EXPO_PUBLIC_API_BASE_URL;
    if (!apiUrl || apiUrl.includes('localhost') || apiUrl.includes('127.0.0.1')) {
      // Skip sync if no real API URL is configured
      return;
    }
    
    try {
      await apiService.put(`/courses/${courseId}/progress`, progress);
    } catch (error: any) {
      // Only log non-network errors (network errors are expected when offline)
      if (error.code !== 'ERR_NETWORK' && error.message !== 'Network Error') {
        logger.error('Error syncing progress:', error);
      }
      // Continue with local storage even if sync fails
    }
  }, [courseId, progress]);

  // Calculate overall progress
  const calculateOverallProgress = useCallback((): number => {
    if (!course || !progress) return 0;
    
    const totalLessons = course.totalLessons;
    if (totalLessons === 0) return 0;
    
    const completedLessons = Object.values(progress.lessons).filter(
      (lp) => lp.completed
    ).length;
    
    return Math.round((completedLessons / totalLessons) * 100);
  }, [course, progress]);

  // Update lesson progress
  const updateLessonProgress = useCallback(
    async (lessonId: string, lessonProgress: Partial<LessonProgress>) => {
      if (!progress) return;

      const existing = progress.lessons[lessonId] || {
        lessonId,
        completed: false,
        lastPosition: 0,
        timeSpent: 0,
      };

      const updated: CourseProgress = {
        ...progress,
        lessons: {
          ...progress.lessons,
          [lessonId]: {
            ...existing,
            ...lessonProgress,
          },
        },
        lastAccessed: new Date().toISOString(),
      };

      updated.overallProgress = calculateOverallProgress();
      await saveProgress(updated);
    },
    [progress, saveProgress, calculateOverallProgress]
  );

  // Mark lesson as complete
  const markLessonComplete = useCallback(
    async (lessonId: string) => {
      await updateLessonProgress(lessonId, {
        completed: true,
        completedAt: new Date().toISOString(),
      });
    },
    [updateLessonProgress]
  );

  // Set current lesson
  const setCurrentLesson = useCallback(
    async (lessonId: string, sectionId: string) => {
      if (!progress) return;

      const updated: CourseProgress = {
        ...progress,
        currentLessonId: lessonId,
        currentSectionId: sectionId,
        lastAccessed: new Date().toISOString(),
      };

      await saveProgress(updated);
    },
    [progress, saveProgress]
  );

  // Update last position
  const updateLastPosition = useCallback(
    async (lessonId: string, position: number) => {
      const existing = progress?.lessons[lessonId];
      await updateLessonProgress(lessonId, {
        lastPosition: position,
        timeSpent: (existing?.timeSpent || 0) + 1, // Increment time spent
      });
    },
    [progress, updateLessonProgress]
  );

  // Add bookmark
  const addBookmark = useCallback(
    async (lessonId: string) => {
      if (!progress) return;

      if (progress.bookmarks.includes(lessonId)) return;

      const updated: CourseProgress = {
        ...progress,
        bookmarks: [...progress.bookmarks, lessonId],
      };

      await saveProgress(updated);
      
      // Sync bookmark to server (only if API is available)
      const apiUrl = process.env.EXPO_PUBLIC_API_BASE_URL;
      if (apiUrl && !apiUrl.includes('localhost') && !apiUrl.includes('127.0.0.1')) {
        try {
          await apiService.post(`/courses/${courseId}/bookmarks`, { lessonId });
        } catch (error: any) {
          if (error.code !== 'ERR_NETWORK' && error.message !== 'Network Error') {
            logger.error('Error syncing bookmark:', error);
          }
        }
      }
    },
    [courseId, progress, saveProgress]
  );

  // Remove bookmark
  const removeBookmark = useCallback(
    async (lessonId: string) => {
      if (!progress) return;

      const updated: CourseProgress = {
        ...progress,
        bookmarks: progress.bookmarks.filter((id) => id !== lessonId),
      };

      await saveProgress(updated);
      
      // Sync bookmark removal to server (only if API is available)
      const apiUrl = process.env.EXPO_PUBLIC_API_BASE_URL;
      if (apiUrl && !apiUrl.includes('localhost') && !apiUrl.includes('127.0.0.1')) {
        try {
          await apiService.delete(`/courses/${courseId}/bookmarks/${lessonId}`);
        } catch (error: any) {
          if (error.code !== 'ERR_NETWORK' && error.message !== 'Network Error') {
            logger.error('Error syncing bookmark removal:', error);
          }
        }
      }
    },
    [courseId, progress, saveProgress]
  );

  // Add note
  const addNote = useCallback(
    async (lessonId: string, content: string, timestamp: number): Promise<Note> => {
      if (!progress) throw new Error('Progress not loaded');

      const note: Note = {
        id: `note_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
        lessonId,
        content,
        timestamp,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      const updated: CourseProgress = {
        ...progress,
        notes: {
          ...progress.notes,
          [lessonId]: [...(progress.notes[lessonId] || []), note],
        },
      };

      await saveProgress(updated);
      
      // Sync note to server (only if API is available)
      const apiUrl = process.env.EXPO_PUBLIC_API_BASE_URL;
      if (apiUrl && !apiUrl.includes('localhost') && !apiUrl.includes('127.0.0.1')) {
        try {
          await apiService.post(`/courses/${courseId}/notes`, note);
        } catch (error: any) {
          if (error.code !== 'ERR_NETWORK' && error.message !== 'Network Error') {
            logger.error('Error syncing note:', error);
          }
        }
      }

      return note;
    },
    [courseId, progress, saveProgress]
  );

  // Update note
  const updateNote = useCallback(
    async (lessonId: string, noteId: string, content: string) => {
      if (!progress) return;

      const lessonNotes = progress.notes[lessonId] || [];
      const updatedNotes = lessonNotes.map((note) =>
        note.id === noteId
          ? { ...note, content, updatedAt: new Date().toISOString() }
          : note
      );

      const updated: CourseProgress = {
        ...progress,
        notes: {
          ...progress.notes,
          [lessonId]: updatedNotes,
        },
      };

      await saveProgress(updated);
      
      // Sync note update to server (only if API is available)
      const apiUrl = process.env.EXPO_PUBLIC_API_BASE_URL;
      if (apiUrl && !apiUrl.includes('localhost') && !apiUrl.includes('127.0.0.1')) {
        try {
          await apiService.put(`/courses/${courseId}/notes/${noteId}`, { content });
        } catch (error: any) {
          if (error.code !== 'ERR_NETWORK' && error.message !== 'Network Error') {
            logger.error('Error syncing note update:', error);
          }
        }
      }
    },
    [courseId, progress, saveProgress]
  );

  // Delete note
  const deleteNote = useCallback(
    async (lessonId: string, noteId: string) => {
      if (!progress) return;

      const lessonNotes = progress.notes[lessonId] || [];
      const updatedNotes = lessonNotes.filter((note) => note.id !== noteId);

      const updated: CourseProgress = {
        ...progress,
        notes: {
          ...progress.notes,
          [lessonId]: updatedNotes,
        },
      };

      await saveProgress(updated);
      
      // Sync note deletion to server (only if API is available)
      const apiUrl = process.env.EXPO_PUBLIC_API_BASE_URL;
      if (apiUrl && !apiUrl.includes('localhost') && !apiUrl.includes('127.0.0.1')) {
        try {
          await apiService.delete(`/courses/${courseId}/notes/${noteId}`);
        } catch (error: any) {
          if (error.code !== 'ERR_NETWORK' && error.message !== 'Network Error') {
            logger.error('Error syncing note deletion:', error);
          }
        }
      }
    },
    [courseId, progress, saveProgress]
  );

  // Load progress on mount
  useEffect(() => {
    loadProgress();
  }, [loadProgress]);

  // Auto-sync with server
  useEffect(() => {
    if (!autoSync || !progress) return;

    const interval = setInterval(() => {
      syncProgress();
    }, SYNC_INTERVAL);

    return () => clearInterval(interval);
  }, [autoSync, progress, syncProgress]);

  // Sync on unmount
  useEffect(() => {
    return () => {
      if (progress) {
        syncProgress();
      }
    };
  }, []);

  return {
    progress,
    isLoading,
    updateLessonProgress,
    markLessonComplete,
    setCurrentLesson,
    addBookmark,
    removeBookmark,
    addNote,
    updateNote,
    deleteNote,
    updateLastPosition,
    calculateOverallProgress,
    syncProgress,
  };
}
