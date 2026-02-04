/**
 * Post service for creating and fetching posts.
 */

import api from './api';

export interface Author {
  id: number;
  username: string;
  displayName: string;
  avatarUrl: string | null;
}

export interface GameData {
  id: string;
  opponent: string;
  opponentRating: number | null;
  userRating: number | null;
  result: string;
  userColor: string;
  timeControl: string | null;
  date: string | null;
  moves: string[];
  tags: string[];
  keyPositionIndex: number;
}

export interface Post {
  id: number;
  author: Author;
  postType: 'text' | 'game_share';
  content: string;
  gameData: GameData | null;
  createdAt: string;
}

export interface PostsResponse {
  posts: Post[];
  total: number;
  hasMore: boolean;
}

export interface CreatePostData {
  content: string;
  postType: 'text' | 'game_share';
  gameId?: number;
  keyPositionIndex?: number;
}

export const postService = {
  /**
   * Get posts feed.
   */
  async getPosts(limit: number = 20, offset: number = 0): Promise<PostsResponse> {
    return api.get<PostsResponse>(`/api/posts?limit=${limit}&offset=${offset}`);
  },

  /**
   * Get posts by a specific user.
   */
  async getUserPosts(username: string, limit: number = 20, offset: number = 0): Promise<PostsResponse> {
    return api.get<PostsResponse>(`/api/users/${username}/posts?limit=${limit}&offset=${offset}`);
  },

  /**
   * Create a new post.
   */
  async createPost(data: CreatePostData): Promise<Post> {
    return api.post<Post>('/api/posts', data);
  },
};

export default postService;
