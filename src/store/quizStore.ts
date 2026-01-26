import { create } from 'zustand';
import AsyncStorage from '@react-native-async-storage/async-storage';
import { Quiz, Question, QuizProgress } from '../types/course';
import logger from '../utils/logger';

const QUIZ_SESSION_KEY = '@teachlink_quiz_session';
const QUIZ_PROGRESS_KEY = '@teachlink_quiz_progress';

interface QuizSession {
  quizId: string | null;
  sectionId: string | null;
  courseId: string | null;
  currentQuestionIndex: number;
  selectedAnswers: Record<string, string | number | (string | number)[]>; // questionId -> answer(s)
  startedAt: string | null;
}

interface QuizState {
  // Session state (temporary, for active quiz)
  session: QuizSession;
  
  // Progress state (persistent, synced with AsyncStorage)
  quizProgress: Record<string, QuizProgress>; // quizId -> QuizProgress
  
  // Actions
  startQuiz: (quizId: string, sectionId: string, courseId: string) => Promise<void>;
  selectAnswer: (questionId: string, answer: string | number, isMultiSelect?: boolean) => void;
  goToQuestion: (index: number) => void;
  completeQuiz: (quiz: Quiz) => Promise<{ score: number; passed: boolean }>;
  resetSession: () => void;
  loadQuizProgress: (courseId: string) => Promise<void>;
  getQuizProgress: (quizId: string) => QuizProgress | null;
  hasCompletedQuiz: (quizId: string) => boolean;
}

const initialSession: QuizSession = {
  quizId: null,
  sectionId: null,
  courseId: null,
  currentQuestionIndex: 0,
  selectedAnswers: {},
  startedAt: null,
};

export const useQuizStore = create<QuizState>((set, get) => ({
  session: initialSession,
  quizProgress: {},

  startQuiz: async (quizId: string, sectionId: string, courseId: string) => {
    try {
      const newSession: QuizSession = {
        quizId,
        sectionId,
        courseId,
        currentQuestionIndex: 0,
        selectedAnswers: {},
        startedAt: new Date().toISOString(),
      };

      set({ session: newSession });

      // Save session to AsyncStorage
      await AsyncStorage.setItem(QUIZ_SESSION_KEY, JSON.stringify(newSession));
      
      logger.info('Quiz started:', { quizId, sectionId, courseId });
    } catch (error) {
      logger.error('Error starting quiz:', error);
      throw error;
    }
  },

  selectAnswer: (questionId: string, answer: string | number, isMultiSelect = false) => {
    const { session } = get();
    let updatedAnswer: string | number | (string | number)[];

    if (isMultiSelect) {
      // Multi-select: toggle answer in/out of array
      const currentAnswer = session.selectedAnswers[questionId];
      const currentArray = Array.isArray(currentAnswer) 
        ? currentAnswer 
        : currentAnswer !== undefined 
          ? [currentAnswer] 
          : [];

      const answerIndex = currentArray.indexOf(answer);
      if (answerIndex > -1) {
        // Remove answer if already selected
        updatedAnswer = currentArray.filter((a) => a !== answer);
        // If array becomes empty, remove the key
        if (updatedAnswer.length === 0) {
          const { [questionId]: _, ...rest } = session.selectedAnswers;
          updatedAnswer = undefined as any;
          const updatedSession: QuizSession = {
            ...session,
            selectedAnswers: rest,
          };
          set({ session: updatedSession });
          AsyncStorage.setItem(QUIZ_SESSION_KEY, JSON.stringify(updatedSession)).catch(
            (error) => logger.error('Error saving quiz session:', error)
          );
          return;
        }
      } else {
        // Add answer if not selected
        updatedAnswer = [...currentArray, answer];
      }
    } else {
      // Single-select: replace answer
      updatedAnswer = answer;
    }

    const updatedAnswers = {
      ...session.selectedAnswers,
      [questionId]: updatedAnswer,
    };

    const updatedSession: QuizSession = {
      ...session,
      selectedAnswers: updatedAnswers,
    };

    set({ session: updatedSession });

    // Auto-save session
    AsyncStorage.setItem(QUIZ_SESSION_KEY, JSON.stringify(updatedSession)).catch(
      (error) => logger.error('Error saving quiz session:', error)
    );
  },

  goToQuestion: (index: number) => {
    const { session } = get();
    if (session.quizId) {
      const updatedSession: QuizSession = {
        ...session,
        currentQuestionIndex: index,
      };
      set({ session: updatedSession });
      
      AsyncStorage.setItem(QUIZ_SESSION_KEY, JSON.stringify(updatedSession)).catch(
        (error) => logger.error('Error saving quiz session:', error)
      );
    }
  },

  completeQuiz: async (quiz: Quiz) => {
    const { session, quizProgress } = get();
    
    if (!session.quizId || !session.courseId) {
      throw new Error('No active quiz session');
    }

    try {
      // Calculate score
      let correctCount = 0;
      let totalPoints = 0;
      let earnedPoints = 0;

      quiz.questions.forEach((question) => {
        totalPoints += question.points;
        const selectedAnswer = session.selectedAnswers[question.id];
        
        if (selectedAnswer !== undefined) {
          let isCorrect = false;

          if (question.multiple) {
            // Multi-select: compare arrays (order-independent)
            const correctAnswers = Array.isArray(question.correctAnswer)
              ? question.correctAnswer
              : [question.correctAnswer];
            const selectedAnswers = Array.isArray(selectedAnswer)
              ? selectedAnswer
              : [selectedAnswer];

            // Check if arrays have same length and all items match
            if (correctAnswers.length === selectedAnswers.length) {
              const correctSorted = [...correctAnswers].sort();
              const selectedSorted = [...selectedAnswers].sort();
              isCorrect = correctSorted.every(
                (val, idx) => val === selectedSorted[idx]
              );
            }
          } else {
            // Single-select: direct comparison
            isCorrect = selectedAnswer === question.correctAnswer;
          }

          if (isCorrect) {
            correctCount++;
            earnedPoints += question.points;
          }
        }
      });

      const score = totalPoints > 0 
        ? Math.round((earnedPoints / totalPoints) * 100) 
        : 0;

      const passed = quiz.passingScore 
        ? score >= quiz.passingScore 
        : score >= 70; // Default passing score

      // Get existing progress or create new
      const existingProgress = quizProgress[session.quizId];
      const newProgress: QuizProgress = {
        quizId: session.quizId,
        sectionId: session.sectionId || '',
        completed: true,
        score,
        answers: { ...session.selectedAnswers },
        completedAt: new Date().toISOString(),
        attempts: (existingProgress?.attempts || 0) + 1,
      };

      // Update state
      const updatedProgress = {
        ...quizProgress,
        [session.quizId]: newProgress,
      };

      set({ quizProgress: updatedProgress });

      // Save to AsyncStorage
      const storageKey = `${QUIZ_PROGRESS_KEY}_${session.courseId}`;
      await AsyncStorage.setItem(storageKey, JSON.stringify(updatedProgress));

      // Clear session
      await AsyncStorage.removeItem(QUIZ_SESSION_KEY);
      set({ session: initialSession });

      logger.info('Quiz completed:', { quizId: session.quizId, score, passed });

      return { score, passed };
    } catch (error) {
      logger.error('Error completing quiz:', error);
      throw error;
    }
  },

  resetSession: async () => {
    try {
      await AsyncStorage.removeItem(QUIZ_SESSION_KEY);
      set({ session: initialSession });
    } catch (error) {
      logger.error('Error resetting quiz session:', error);
    }
  },

  loadQuizProgress: async (courseId: string) => {
    try {
      const storageKey = `${QUIZ_PROGRESS_KEY}_${courseId}`;
      const stored = await AsyncStorage.getItem(storageKey);
      
      if (stored) {
        const parsed = JSON.parse(stored) as Record<string, QuizProgress>;
        set({ quizProgress: parsed });
      } else {
        set({ quizProgress: {} });
      }

      // Also try to restore active session if exists
      const sessionData = await AsyncStorage.getItem(QUIZ_SESSION_KEY);
      if (sessionData) {
        const session = JSON.parse(sessionData) as QuizSession;
        // Only restore if it's for the current course
        if (session.courseId === courseId) {
          set({ session });
        } else {
          // Clear stale session
          await AsyncStorage.removeItem(QUIZ_SESSION_KEY);
        }
      }
    } catch (error) {
      logger.error('Error loading quiz progress:', error);
      set({ quizProgress: {} });
    }
  },

  getQuizProgress: (quizId: string) => {
    const { quizProgress } = get();
    return quizProgress[quizId] || null;
  },

  hasCompletedQuiz: (quizId: string) => {
    const { quizProgress } = get();
    return quizProgress[quizId]?.completed || false;
  },
}));
