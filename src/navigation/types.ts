import { Course } from '../types/course';

export type RootStackParamList = {
  Home: undefined;
  Profile: { userId: string };
  Settings: undefined;
  CourseViewer: { course: Course; initialLessonId?: string };
};
