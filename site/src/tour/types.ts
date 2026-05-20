export interface Lesson {
  id: string;
  title: string;
  instructions: string;
  hints: string[];
  initialCode: string;
}

export interface Chapter {
  id: string;
  title: string;
  lessons: Lesson[];
}
