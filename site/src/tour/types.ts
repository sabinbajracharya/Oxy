export interface Lesson {
  id: string;
  title: string;
  instructions: string;
  hints: string[];
  initialCode: string;
  testCode: string;
}

export interface Chapter {
  id: string;
  title: string;
  lessons: Lesson[];
}
